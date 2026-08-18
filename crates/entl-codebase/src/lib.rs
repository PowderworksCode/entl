//! Typed inventory and profiles for source codebases.
//!
//! `entl-codebase` walks a codebase once and returns reusable facts about its
//! files, languages, manifests, packages, and workspaces. It does not enforce
//! policy and it does not eagerly read complete file contents. Consumers such
//! as linters and codebase auditors decide what the facts mean.

mod compiler;
mod discovery;
mod error;
mod model;
mod walk;

pub use compiler::{CompilerObservation, observe_rust_compiler};
pub use discovery::{
    DiscoveryBuilder, DiscoveryHandler, DiscoveryHandlerRegistration, DiscoveryPhase,
    discovery_handlers, inspect, registry as discovery_registry,
};
pub use error::{Error, Result};
pub use model::{
    Artifact, CodebaseInventory, CodebaseTree, Dependency, DependencyKind, DependencyResolution,
    DependencySource, Diagnostic, DiagnosticKind, FileEntry, Manifest, ManifestKind, Package,
    PackageId, PackageKind, PackageLanguage, PackageScript, Project, ResolvedPackage, Workspace,
    WorkspaceId, WorkspaceKind,
};
// No language vocabulary is re-exported here. The registries that used to be
// src/profiles/ are langbank's — 827 languages against this module's eight,
// checked against eleven upstreams on a schedule — and code that needs a
// LanguageProfile, an EcosystemProfile or a detection takes it from langbank
// directly. entl's public surface is what entl owns: walking, inventory,
// manifests, packages, workspaces.
pub use walk::{InventoryOptions, walk};
