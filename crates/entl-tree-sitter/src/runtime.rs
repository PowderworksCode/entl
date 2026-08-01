use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tree_sitter::{
    Language, Node, Parser, Query, QueryCursor, StreamingIterator, Tree, WasmStore,
    wasmtime::Engine,
};

use crate::{Error, ParserPack, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProvenance {
    pub parser_id: String,
    pub parser_version: String,
    pub grammar_sha256: String,
    pub source_sha256: String,
}

#[derive(Debug)]
pub struct ParsedFile {
    /// Why the source had to be rewritten before the grammar would read it.
    ///
    /// Empty for source the grammar accepts as written. A consumer that cares
    /// whether it is looking at the file as the author wrote it can check.
    pub rewrites: Vec<&'static str>,
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
    engine: Engine,
    pack: Arc<ParserPack>,
    language: Language,
    queries: BTreeMap<String, Query>,
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
        let mut parser = Parser::new();
        let store =
            WasmStore::new(&self.engine).map_err(|error| Error::Runtime(error.to_string()))?;
        parser
            .set_wasm_store(store)
            .map_err(|error| Error::ConfigureParser {
                pack: self.pack.manifest().id.clone(),
                message: error.to_string(),
            })?;
        parser
            .set_language(&self.language)
            .map_err(|error| Error::ConfigureParser {
                pack: self.pack.manifest().id.clone(),
                message: error.to_string(),
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
        if tree.root_node().has_error()
            && let Some(rewritten) =
                crate::dialect::neutralize(self.pack.language().id, source.as_ref())
            && let Some(retried) = parser.parse(rewritten.source.as_slice(), None)
            && !retried.root_node().has_error()
        {
            source = Arc::<[u8]>::from(rewritten.source);
            rewrites = rewritten.reasons;
            tree = retried;
        }
        Ok(ParsedFile {
            path,
            provenance: ParseProvenance {
                parser_id: self.pack.manifest().id.clone(),
                parser_version: self.pack.manifest().version.clone(),
                grammar_sha256: self.pack.manifest().sha256.clone(),
                source_sha256: hex_digest(&source),
            },
            pack: self.pack.clone(),
            source,
            tree,
            rewrites,
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
        // Compile here, where the failure can still stop the load. A query
        // that does not compile would otherwise match nothing, and a rule that
        // matches nothing reports nothing, which reads as a clean repository.
        let mut queries = BTreeMap::new();
        for (name, source) in pack.queries() {
            let query =
                Query::new(&language, source).map_err(|error| Error::CompileQuery {
                    pack: pack.manifest().id.clone(),
                    query: name.clone(),
                    message: format!(
                        "at row {}, offset {}: {}",
                        error.row, error.offset, error.message
                    ),
                })?;
            queries.insert(name.clone(), query);
        }
        Ok(LoadedParser {
            engine: self.engine.clone(),
            pack,
            language,
            queries,
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
