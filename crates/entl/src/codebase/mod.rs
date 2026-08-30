//! Typed inventory and profiles for source codebases.
//!
//! `entl` walks a codebase once and returns reusable facts about its
//! files, languages, manifests, packages, and workspaces. It does not enforce
//! policy and it does not eagerly read complete file contents. Consumers such
//! as linters and codebase auditors decide what the facts mean.

mod compiler;
mod discovery;
mod error;
mod model;
mod profiles;
mod walk;

pub use compiler::{CompilerObservation, observe_rust_compiler};
pub use discovery::{
    DiscoveryBuilder, DiscoveryHandler, DiscoveryHandlerRegistration, DiscoveryPhase,
    discovery_handlers, inspect, registry as discovery_registry,
};
pub use error::{Error, Result};
pub use model::{
    Artifact, ArtifactId, CodebaseInventory, CodebaseTree, Dependency, DependencyKind,
    DependencyResolution, DependencySource, Diagnostic, DiagnosticKind, EcosystemId, FileEntry,
    LanguageDetection, LanguageEvidence, LanguageId, Manifest, ManifestKind, Package, PackageId,
    PackageKind, PackageLanguage, PackageScript, Project, ProjectFacetId, ResolvedPackage,
    Workspace, WorkspaceId, WorkspaceKind,
};
pub use profiles::{
    ArgumentPattern, ArtifactProfile, ArtifactRegistration, BINARY_ARTIFACT, BUN_ECOSYSTEM,
    CARGO_ECOSYSTEM, CODESPELL, COMPONENT_HOST, CSS_LANGUAGE, CiWorkload, CommandPattern,
    CommentSyntax, DependencyPinPolicy, DependencyPinStatus, DependencyPinSyntax, EcosystemProfile,
    EcosystemRegistration, EcosystemRole, HAWK, JAVASCRIPT_LANGUAGE, LESS_LANGUAGE,
    LanguageConventions, LanguageFacet, LanguageFacetRegistration, LanguageProfile,
    LanguageRegistration, LanguageRole, LanguageVerbosity, ManifestSelection, NAPI_ARTIFACT,
    NPM_ECOSYSTEM, PNPM_ECOSYSTEM, RUST_LANGUAGE, SCSS_LANGUAGE, SHELL_LANGUAGE, SHELLCHECK,
    SITE_ARTIFACT, STRUCTURED_CODE, STYLE_HOST, STYLELINT, TAURI_ARTIFACT, TYPESCRIPT_LANGUAGE,
    TaskKind, TestLayoutDefaults, TestRetryConfiguration, TestRetryProfile, TestRetrySignal,
    ToolId, ToolProfile, ToolRegistration, TraversalDirectory, TraversalDirectoryRegistration,
    TypecheckConvention, VALE, VERBOSITY_BASELINE, VERBOSITY_CORPUS, VERBOSITY_CORPUS_REVISION,
    VerbosityRatio, YARN_ECOSYSTEM, ZIZMOR, artifact_profile, artifact_profiles, classify_tool,
    comment_syntax, comment_syntax_for_extension, detect_language, ecosystem_profile,
    ecosystem_profiles, language_conventions, language_facet, language_facets, language_profile,
    language_profile_for_extension, language_profiles, normalize_invocation,
    registry as profile_registry, tool_profile, tool_profiles, traversal_directories, verbosity,
    verbosity_ratio, verbosity_ratios,
};
pub use walk::{InventoryOptions, walk};
