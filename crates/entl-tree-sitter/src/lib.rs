//! Runtime-loadable Tree-sitter grammars for Entl codebase consumers.
//!
//! Grammar implementations are Wasm parser packs discovered at runtime. This
//! crate owns acquisition and parse provenance; consumers own interpretations
//! of the resulting concrete syntax trees.

mod catalog;
mod dialect;
mod error;
mod manifest;
mod repository;
mod runtime;

pub use catalog::{CatalogDiscovery, ParserCatalog, ParserPack};
pub use dialect::{Rewritten, neutralize};
pub use error::{Error, Result};
pub use manifest::{
    FileSelectionManifest, MANIFEST_FILENAME, ParserPackManifest, TokenizationManifest,
};
pub use repository::{ParseDiagnostic, ParsedRepository, parse_repository};
pub use runtime::{LoadedParser, ParseProvenance, ParsedFile, ParserRuntime};
