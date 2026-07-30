use std::path::PathBuf;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tree_sitter::{Language, Parser, Tree, WasmStore, wasmtime::Engine};

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
    pub path: PathBuf,
    pub source: Arc<[u8]>,
    pub tree: Tree,
    pub pack: Arc<ParserPack>,
    pub provenance: ParseProvenance,
}

pub struct LoadedParser {
    engine: Engine,
    pack: Arc<ParserPack>,
    language: Language,
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
        let tree = parser
            .parse(source.as_ref(), None)
            .ok_or_else(|| Error::ParseCancelled { path: path.clone() })?;

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
        Ok(LoadedParser {
            engine: self.engine.clone(),
            pack,
            language,
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
