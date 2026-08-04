//! Writing the AIR stream to Parquet, a row group at a time.
//!
//! Two tables, because they answer different questions at wildly different
//! sizes: `functions` is one row per function and small enough to scan, and
//! `instructions` is one row per AIR instruction and is not.
//!
//! ## Why Parquet
//!
//! The dump is write-once, read-many, and every question asked of it touches a
//! few columns of a great many rows — "every `store_safe` whose target is a
//! `struct_field_ptr_index_*`" reads three columns out of ten. That is what
//! column pruning and row-group statistics are for, and it means DuckDB or
//! Polars can query the output in place with no loader.
//!
//! `op` and `result_type` are dictionary-encoded. There are a few hundred
//! distinct operations and, in a real corpus, far fewer distinct types than
//! instructions, so the dictionary is both the compression win and what makes
//! a predicate on either column cheap.
//!
//! ## Why it streams
//!
//! A 20-line Zig file produces 6.7MB of AIR across 1,178 functions, nearly all
//! of it `std`. Bun has some 30,000 functions of its own. Rows are buffered to
//! a row group and flushed, so memory is bounded by the row-group size and not
//! by the corpus.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, ListBuilder, StringBuilder, StringDictionaryBuilder, UInt8Builder,
    UInt32Builder,
};
use arrow_array::types::UInt16Type;
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

use crate::air::Function;

/// How many rows accumulate before a row group is written.
///
/// Row groups are the unit of statistics and of parallel reading, so this is a
/// trade: larger groups compress better, smaller ones let a reader skip more
/// precisely. 128k instruction rows is a few megabytes.
const ROW_GROUP: usize = 128 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("writing {}: {source}", path.display())]
    Parquet {
        path: std::path::PathBuf,
        #[source]
        source: parquet::errors::ParquetError,
    },
    #[error("creating {}: {source}", path.display())]
    Create {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("assembling a record batch: {0}")]
    Batch(#[from] arrow_schema::ArrowError),
}

pub type Result<T> = std::result::Result<T, Error>;

fn properties() -> WriterProperties {
    WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build()
}

fn functions_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt32, false),
        // The compiler's own name. See `air::Function::mangled` on why a file
        // is a join against known paths rather than a split.
        Field::new("mangled", DataType::Utf8, false),
        Field::new("declared_instructions", DataType::UInt32, false),
        Field::new("read_instructions", DataType::UInt32, false),
        Field::new("air_bytes", DataType::UInt32, false),
    ]))
}

fn instructions_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("function", DataType::UInt32, false),
        Field::new("index", DataType::UInt32, false),
        Field::new(
            "op",
            DataType::Dictionary(Box::new(DataType::UInt16), Box::new(DataType::Utf8)),
            false,
        ),
        Field::new(
            "result_type",
            DataType::Dictionary(Box::new(DataType::UInt16), Box::new(DataType::Utf8)),
            false,
        ),
        Field::new("arguments", DataType::Utf8, false),
        // The list is always present; its items are declared nullable because
        // that is what the builder produces, and a schema that lies about its
        // own data is worse than a loose one.
        Field::new(
            "operands",
            DataType::List(Arc::new(Field::new("item", DataType::UInt32, true))),
            false,
        ),
        Field::new("dead", DataType::Boolean, false),
        Field::new("depth", DataType::UInt8, false),
        Field::new("line", DataType::UInt32, false),
        Field::new("column", DataType::UInt32, false),
    ]))
}

/// Accumulates functions and their instructions into two Parquet files.
pub struct Store {
    functions: ArrowWriter<File>,
    instructions: ArrowWriter<File>,
    functions_path: std::path::PathBuf,
    instructions_path: std::path::PathBuf,
    pending: Pending,
    next_id: u32,
    rows: usize,
    pub written_functions: usize,
    pub written_instructions: usize,
}

#[derive(Default)]
struct Pending {
    function_id: UInt32Builder,
    mangled: StringBuilder,
    declared: UInt32Builder,
    read: UInt32Builder,
    air_bytes: UInt32Builder,

    owner: UInt32Builder,
    index: UInt32Builder,
    op: StringDictionaryBuilder<UInt16Type>,
    result_type: StringDictionaryBuilder<UInt16Type>,
    arguments: StringBuilder,
    operands: ListBuilder<UInt32Builder>,
    dead: BooleanBuilder,
    depth: UInt8Builder,
    line: UInt32Builder,
    column: UInt32Builder,
}

impl Store {
    pub fn create(directory: &Path) -> Result<Self> {
        std::fs::create_dir_all(directory).map_err(|source| Error::Create {
            path: directory.to_path_buf(),
            source,
        })?;
        let functions_path = directory.join("functions.parquet");
        let instructions_path = directory.join("instructions.parquet");
        let functions = writer(&functions_path, functions_schema())?;
        let instructions = writer(&instructions_path, instructions_schema())?;
        Ok(Store {
            functions,
            instructions,
            functions_path,
            instructions_path,
            pending: Pending::default(),
            next_id: 0,
            rows: 0,
            written_functions: 0,
            written_instructions: 0,
        })
    }

    pub fn push(&mut self, function: &Function) -> Result<()> {
        let id = self.next_id;
        self.next_id += 1;

        self.pending.function_id.append_value(id);
        self.pending.mangled.append_value(&function.mangled);
        self.pending
            .declared
            .append_value(function.declared_instructions);
        self.pending
            .read
            .append_value(u32::try_from(function.instructions.len()).unwrap_or(u32::MAX));
        self.pending.air_bytes.append_value(function.air_bytes);
        self.written_functions += 1;

        for instruction in &function.instructions {
            self.pending.owner.append_value(id);
            self.pending.index.append_value(instruction.index);
            self.pending.op.append_value(&instruction.op);
            self.pending
                .result_type
                .append_value(&instruction.result_type);
            self.pending.arguments.append_value(&instruction.arguments);
            let operands = self.pending.operands.values();
            for operand in &instruction.operands {
                operands.append_value(*operand);
            }
            self.pending.operands.append(true);
            self.pending.dead.append_value(instruction.dead);
            self.pending.depth.append_value(instruction.depth);
            self.pending.line.append_value(instruction.line);
            self.pending.column.append_value(instruction.column);
            self.rows += 1;
            self.written_instructions += 1;
        }

        if self.rows >= ROW_GROUP {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if self.written_functions == 0 && self.rows == 0 {
            return Ok(());
        }
        let functions: Vec<ArrayRef> = vec![
            Arc::new(self.pending.function_id.finish()),
            Arc::new(self.pending.mangled.finish()),
            Arc::new(self.pending.declared.finish()),
            Arc::new(self.pending.read.finish()),
            Arc::new(self.pending.air_bytes.finish()),
        ];
        if !functions[0].is_empty() {
            let batch = RecordBatch::try_new(functions_schema(), functions)?;
            self.functions
                .write(&batch)
                .map_err(|source| Error::Parquet {
                    path: self.functions_path.clone(),
                    source,
                })?;
        }

        let instructions: Vec<ArrayRef> = vec![
            Arc::new(self.pending.owner.finish()),
            Arc::new(self.pending.index.finish()),
            Arc::new(self.pending.op.finish()),
            Arc::new(self.pending.result_type.finish()),
            Arc::new(self.pending.arguments.finish()),
            Arc::new(self.pending.operands.finish()),
            Arc::new(self.pending.dead.finish()),
            Arc::new(self.pending.depth.finish()),
            Arc::new(self.pending.line.finish()),
            Arc::new(self.pending.column.finish()),
        ];
        if !instructions[0].is_empty() {
            let batch = RecordBatch::try_new(instructions_schema(), instructions)?;
            self.instructions
                .write(&batch)
                .map_err(|source| Error::Parquet {
                    path: self.instructions_path.clone(),
                    source,
                })?;
        }
        self.rows = 0;
        Ok(())
    }

    pub fn close(mut self) -> Result<()> {
        self.flush()?;
        self.functions.close().map_err(|source| Error::Parquet {
            path: self.functions_path.clone(),
            source,
        })?;
        self.instructions.close().map_err(|source| Error::Parquet {
            path: self.instructions_path.clone(),
            source,
        })?;
        Ok(())
    }
}

fn writer(path: &Path, schema: SchemaRef) -> Result<ArrowWriter<File>> {
    let file = File::create(path).map_err(|source| Error::Create {
        path: path.to_path_buf(),
        source,
    })?;
    ArrowWriter::try_new(file, schema, Some(properties())).map_err(|source| Error::Parquet {
        path: path.to_path_buf(),
        source,
    })
}
