use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Codebase(#[from] entl_codebase::Error),

    #[error("could not read parser pack file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not parse parser pack manifest {path}: {source}")]
    Manifest {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("parser pack {pack:?} uses unsupported manifest schema {schema}")]
    UnsupportedSchema { pack: String, schema: u32 },

    #[error("parser pack path {path:?} must be relative and may not contain parent components")]
    UnsafePath { path: PathBuf },

    #[error("parser pack {pack:?} references unknown Entl language {language:?}")]
    UnknownLanguage { pack: String, language: String },

    #[error("parser pack {pack:?} has invalid file selector {selector:?}")]
    InvalidFileSelector { pack: String, selector: String },

    #[error("parser pack {pack:?} has invalid SHA-256 digest {digest:?}")]
    InvalidDigest { pack: String, digest: String },

    #[error("parser pack {pack:?} grammar digest mismatch: expected {expected}, found {actual}")]
    DigestMismatch {
        pack: String,
        expected: String,
        actual: String,
    },

    #[error("parser packs for language {language:?} overlap: {first:?} and {second:?}")]
    OverlappingPacks {
        language: String,
        first: String,
        second: String,
    },

    #[error("could not initialize the Tree-sitter Wasm runtime: {0}")]
    Runtime(String),

    #[error("could not load grammar from parser pack {pack:?}: {message}")]
    LoadGrammar { pack: String, message: String },

    #[error("parser pack {pack:?} declares ABI {declared}, but the grammar reports ABI {actual}")]
    AbiMismatch {
        pack: String,
        declared: usize,
        actual: usize,
    },

    #[error("could not configure parser pack {pack:?}: {message}")]
    ConfigureParser { pack: String, message: String },

    #[error("parser pack {pack:?} query {query:?} does not compile: {message}")]
    CompileQuery {
        pack: String,
        query: String,
        message: String,
    },

    #[error("parser pack {pack:?} has no query named {query:?}; it has [{available}]")]
    UnknownQuery {
        pack: String,
        query: String,
        available: String,
    },

    #[error("Tree-sitter cancelled parsing {path}")]
    ParseCancelled { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, Error>;
