use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};
use tree_sitter::{
    Node, Parser, Query, QueryCursor, StreamingIterator, Tree, WasmStore, wasmtime::Engine,
};

use crate::{Error, ParserPack, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProvenance {
    pub parser_id: String,
    pub parser_version: String,
    pub grammar_sha256: String,
    /// A digest over the queries the pack shipped.
    ///
    /// A fact derived through a query depends on that query's text as much as
    /// on the grammar, so recording only `grammar_sha256` cannot say which
    /// rules produced it. A pack with no queries still has a stable digest.
    pub queries_sha256: String,
    pub source_sha256: String,
}

#[derive(Debug)]
pub struct ParsedFile {
    /// Why the source had to be rewritten before the grammar would read it.
    ///
    /// Empty for source the grammar accepts as written. A consumer that cares
    /// whether it is looking at the file as the author wrote it can check.
    pub rewrites: Vec<&'static str>,
    /// Whether a rewrite changed what the source says rather than only what the
    /// grammar could read.
    ///
    /// Every rewrite preserves byte length, so a span reported against this
    /// file is correct either way. The text at that span is not: a rewrite that
    /// had to choose between two comptime-conditional types produced a
    /// signature narrower than the one in the file, and quoting it as the
    /// author's would be a false claim.
    pub rewrites_narrowed: bool,
    pub path: PathBuf,
    pub source: Arc<[u8]>,
    pub tree: Tree,
    pub pack: Arc<ParserPack>,
    pub provenance: ParseProvenance,
}

/// One node a query captured, under the name the query gave it.
#[derive(Debug, Clone, Copy)]
pub struct QueryCapture<'a> {
    pub name: &'a str,
    pub node: Node<'a>,
}

/// One match of a query against a tree.
///
/// A capture that a pattern marks optional is simply absent when it did not
/// match, which is how a query says "no binding here" — Tree-sitter queries
/// have no negation, so absence is the only way to express it.
#[derive(Debug, Clone)]
pub struct QueryMatch<'a> {
    pub pattern: usize,
    pub captures: Vec<QueryCapture<'a>>,
}

impl<'a> QueryMatch<'a> {
    pub fn capture(&self, name: &str) -> Option<Node<'a>> {
        self.captures
            .iter()
            .find(|capture| capture.name == name)
            .map(|capture| capture.node)
    }

    pub fn has(&self, name: &str) -> bool {
        self.captures.iter().any(|capture| capture.name == name)
    }
}

pub struct LoadedParser {
    pack: Arc<ParserPack>,
    queries: BTreeMap<String, Query>,
    /// A parser with its Wasm store and language already configured.
    ///
    /// Building one costs about 123ms regardless of the file: measured, a
    /// 19-byte source took 123ms to parse and a 40KB source took 210ms, so
    /// nearly all of the smaller number was setup being repeated. Doing it per
    /// file put 128 seconds of pure overhead in front of a thousand-file
    /// repository before any work happened. It is held behind a lock because
    /// `Parser::parse` needs `&mut`, while every consumer holds this by
    /// reference and some hold it inside a shared `ParsedRepository`.
    parser: Mutex<Parser>,
}

impl std::fmt::Debug for LoadedParser {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LoadedParser")
            .field("pack", &self.pack.manifest().id)
            .finish_non_exhaustive()
    }
}

impl LoadedParser {
    pub fn pack(&self) -> &ParserPack {
        &self.pack
    }

    /// A compiled query this pack ships, by name.
    pub fn query(&self, name: &str) -> Option<&Query> {
        self.queries.get(name)
    }

    pub fn query_names(&self) -> impl Iterator<Item = &str> {
        self.queries.keys().map(String::as_str)
    }

    /// Run one of this pack's queries over a parsed file.
    ///
    /// Asking for a query the pack does not ship is an error rather than an
    /// empty result: a consumer that named the wrong query would otherwise see
    /// what a genuinely clean file looks like.
    pub fn matches<'a>(&'a self, name: &str, file: &'a ParsedFile) -> Result<Vec<QueryMatch<'a>>> {
        let query = self.queries.get(name).ok_or_else(|| Error::UnknownQuery {
            pack: self.pack.manifest().id.clone(),
            query: name.to_owned(),
            available: self.query_names().collect::<Vec<_>>().join(", "),
        })?;
        let names = query.capture_names();
        let mut cursor = QueryCursor::new();
        let mut found = Vec::new();
        let mut matches = cursor.matches(query, file.tree.root_node(), file.source.as_ref());
        while let Some(matched) = StreamingIterator::next(&mut matches) {
            found.push(QueryMatch {
                pattern: matched.pattern_index,
                captures: matched
                    .captures
                    .iter()
                    .map(|capture| QueryCapture {
                        name: names[capture.index as usize],
                        node: capture.node,
                    })
                    .collect(),
            });
        }
        Ok(found)
    }

    pub fn parse(
        &self,
        path: impl Into<PathBuf>,
        source: impl Into<Arc<[u8]>>,
    ) -> Result<ParsedFile> {
        let path = path.into();
        let source = source.into();
        let mut parser = self.parser.lock().map_err(|error| Error::ConfigureParser {
            pack: self.pack.manifest().id.clone(),
            message: format!("the parser is unusable after an earlier panic: {error}"),
        })?;
        let mut tree = parser
            .parse(source.as_ref(), None)
            .ok_or_else(|| Error::ParseCancelled { path: path.clone() })?;

        // A grammar rejects a whole file when any part of it is beyond what it
        // knows, so a single unsupported keyword removes everything beside it.
        // Retrying without that keyword recovers the rest. Only source that
        // already failed is rewritten, so an accepted file is never altered.
        let mut source = source;
        let mut rewrites = Vec::new();
        let mut rewrites_narrowed = false;
        if tree.root_node().has_error()
            && let Some(rewritten) =
                crate::dialect::neutralize(self.pack.language().id, source.as_ref())
            && let Some(retried) = parser.parse(rewritten.source.as_slice(), None)
            && !retried.root_node().has_error()
        {
            source = Arc::<[u8]>::from(rewritten.source);
            rewrites = rewritten.reasons;
            rewrites_narrowed = rewritten.narrowed;
            tree = retried;
        }
        Ok(ParsedFile {
            path,
            provenance: ParseProvenance {
                parser_id: self.pack.manifest().id.clone(),
                parser_version: self.pack.manifest().version.clone(),
                grammar_sha256: self.pack.manifest().sha256.clone(),
                queries_sha256: self.pack.queries_sha256().to_owned(),
                source_sha256: hex_digest(&source),
            },
            pack: self.pack.clone(),
            source,
            tree,
            rewrites,
            rewrites_narrowed,
        })
    }
}

#[derive(Clone)]
pub struct ParserRuntime {
    engine: Engine,
}

impl std::fmt::Debug for ParserRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParserRuntime")
            .finish_non_exhaustive()
    }
}

impl ParserRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            engine: Engine::default(),
        })
    }

    pub fn load(&self, pack: Arc<ParserPack>) -> Result<LoadedParser> {
        let mut store =
            WasmStore::new(&self.engine).map_err(|error| Error::Runtime(error.to_string()))?;
        let language = store
            .load_language(pack.manifest().grammar_name(), pack.grammar())
            .map_err(|error| Error::LoadGrammar {
                pack: pack.manifest().id.clone(),
                message: error.to_string(),
            })?;
        let actual = language.abi_version();
        if actual != pack.manifest().abi {
            return Err(Error::AbiMismatch {
                pack: pack.manifest().id.clone(),
                declared: pack.manifest().abi,
                actual,
            });
        }
        // A tokenization kind the grammar has never heard of fails exactly the
        // way an uncompiled query would: it matches nothing, silently, and the
        // pack reads as if it had declared nothing at all. The zig pack
        // declared `line_comment`, `doc_comment`, `container_doc_comment` and
        // `field_identifier`, none of which `tree-sitter-zig` 1.1.2 has, so
        // every Zig comment was being compared as code and nobody could tell.
        let tokenization = &pack.manifest().tokenization;
        for (field, kinds) in [
            ("ignored-node-kinds", &tokenization.ignored_node_kinds),
            ("identifier-node-kinds", &tokenization.identifier_node_kinds),
            ("literal-node-kinds", &tokenization.literal_node_kinds),
            ("unit-node-kinds", &tokenization.unit_node_kinds),
        ] {
            for kind in kinds {
                if language.id_for_node_kind(kind, true) == 0
                    && language.id_for_node_kind(kind, false) == 0
                {
                    return Err(Error::UnknownNodeKind {
                        pack: pack.manifest().id.clone(),
                        field: field.to_owned(),
                        kind: kind.clone(),
                    });
                }
            }
        }

        // Compile here, where the failure can still stop the load. A query
        // that does not compile would otherwise match nothing, and a rule that
        // matches nothing reports nothing, which reads as a clean repository.
        let mut queries = BTreeMap::new();
        for (name, source) in pack.queries() {
            let query = Query::new(&language, source).map_err(|error| Error::CompileQuery {
                pack: pack.manifest().id.clone(),
                query: name.clone(),
                message: format!(
                    "at row {}, offset {}: {}",
                    error.row, error.offset, error.message
                ),
            })?;
            queries.insert(name.clone(), query);
        }
        // Configure the parser once, here, rather than on every parse.
        let mut parser = Parser::new();
        let store =
            WasmStore::new(&self.engine).map_err(|error| Error::Runtime(error.to_string()))?;
        parser
            .set_wasm_store(store)
            .map_err(|error| Error::ConfigureParser {
                pack: pack.manifest().id.clone(),
                message: error.to_string(),
            })?;
        parser
            .set_language(&language)
            .map_err(|error| Error::ConfigureParser {
                pack: pack.manifest().id.clone(),
                message: error.to_string(),
            })?;

        Ok(LoadedParser {
            pack,
            queries,
            parser: Mutex::new(parser),
        })
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
