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
    ErrorHandlingManifest, FileSelectionManifest, MANIFEST_FILENAME, ParserPackManifest,
    Propagation, TestManifest, TokenizationManifest,
};
pub use repository::{ParseDiagnostic, ParsedRepository, parse_repository};
pub use runtime::{
    LoadedParser, ParseProvenance, ParsedFile, ParserRuntime, QueryCapture, QueryMatch,
};

/// Whether a consumer that parses source should expect a parser pack for a
/// language of this role, and so should hear about one being missing.
///
/// Reporting a missing pack exists so nothing calls a repository clean when a
/// whole language went unread. That is worth saying for Go or C++, which are
/// real gaps in coverage. It is not worth saying for a README, a
/// `.gitignore`-adjacent YAML file, or the `straitjacket.toml` a run was
/// configured by: those are not the source under analysis, and reporting them
/// buries the gaps that matter.
///
/// The pack format draws the same line. A manifest declares `unit-node-kinds`
/// (`function_item`, `impl_item`), `error-handling` with its fallible and
/// optional types, and `tests` markers, and ships queries for callables and
/// behaviors. None of that has a meaning in TOML. A pack for a data language
/// could only be a stub that exists to silence this.
///
/// Markup and stylesheets sit outside for now because no pack covers them.
/// Vendoring one is the moment to move it, not before.
///
/// An extension trait rather than a method on `LanguageRole` because the role
/// is langbank's type and this is not langbank's judgement: whether a parser
/// pack exists for a role is a fact about *this* crate's pack format, and it
/// belongs where the packs do.
pub trait ExpectsParserPack {
    fn expects_parser_pack(self) -> bool;
}

impl ExpectsParserPack for langbank::LanguageRole {
    fn expects_parser_pack(self) -> bool {
        matches!(self, Self::Programming)
    }
}
