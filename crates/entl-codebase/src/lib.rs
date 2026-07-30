//! Typed inventory and profiles for source codebases.
//!
//! `entl-codebase` walks a codebase once and returns reusable facts about its
//! files, languages, manifests, packages, and workspaces. It does not enforce
//! policy and it does not eagerly read complete file contents. Consumers such
//! as linters and codebase auditors decide what the facts mean.

mod discovery;
mod error;
mod model;
mod profiles;
mod walk;

pub use discovery::{
    DiscoveryBuilder, DiscoveryHandler, DiscoveryHandlerRegistration, DiscoveryPhase,
    discovery_handlers, inspect, registry as discovery_registry,
};
pub use error::{Error, Result};
pub use model::{
    Artifact, ArtifactId, CodebaseInventory, CodebaseTree, Dependency, DependencyKind, Diagnostic,
    DiagnosticKind, EcosystemId, FileEntry, LanguageDetection, LanguageEvidence, LanguageId,
    Manifest, ManifestKind, Package, PackageId, PackageKind, PackageLanguage, PackageScript,
    Project, ProjectFacetId, Workspace, WorkspaceId, WorkspaceKind,
};
pub use profiles::{
    ArgumentPattern, ArtifactProfile, ArtifactRegistration, BINARY_ARTIFACT, BUN_ECOSYSTEM,
    CARGO_ECOSYSTEM, CODESPELL, COMPONENT_HOST, CommandPattern, CommentSyntax, EcosystemProfile,
    EcosystemRegistration, EcosystemRole, JAVASCRIPT_LANGUAGE, LanguageConventions, LanguageFacet,
    LanguageFacetRegistration, LanguageProfile, LanguageRegistration, LanguageRole,
    ManifestSelection, NAPI_ARTIFACT, NPM_ECOSYSTEM, PNPM_ECOSYSTEM, RUST_LANGUAGE, SHELL_LANGUAGE,
    SITE_ARTIFACT, STRUCTURED_CODE, STYLE_HOST, TAURI_ARTIFACT, TYPESCRIPT_LANGUAGE, TaskKind,
    TestLayoutDefaults, ToolId, ToolProfile, ToolRegistration, TraversalDirectory,
    TraversalDirectoryRegistration, TypecheckConvention, VALE, YARN_ECOSYSTEM, artifact_profile,
    artifact_profiles, classify_tool, comment_syntax, comment_syntax_for_extension,
    detect_language, ecosystem_profile, ecosystem_profiles, language_conventions, language_facet,
    language_facets, language_profile, language_profile_for_extension, language_profiles,
    normalize_invocation, registry as profile_registry, tool_profile, tool_profiles,
    traversal_directories,
};
pub use walk::{InventoryOptions, walk};
