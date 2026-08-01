mod artifact;
mod artifacts;
mod convention;
mod ecosystem;
pub(crate) mod ecosystems;
mod facet;
mod facets;
mod language;
pub(crate) mod languages;
mod tool;
mod tools;
mod traversal;
mod verbosity;

pub use artifact::{ArtifactProfile, ArtifactRegistration, artifact_profile, artifact_profiles};
pub use artifacts::{BINARY_ARTIFACT, NAPI_ARTIFACT, SITE_ARTIFACT, TAURI_ARTIFACT};
pub use ecosystem::{
    DependencyPinPolicy, DependencyPinStatus, DependencyPinSyntax, EcosystemProfile,
    EcosystemRegistration, EcosystemRole, ManifestSelection, ecosystem_profile, ecosystem_profiles,
};
pub use ecosystems::{
    BUN as BUN_ECOSYSTEM, CARGO as CARGO_ECOSYSTEM, NPM as NPM_ECOSYSTEM, PNPM as PNPM_ECOSYSTEM,
    YARN as YARN_ECOSYSTEM,
};
pub use facet::{LanguageFacet, LanguageFacetRegistration, language_facet, language_facets};
pub use facets::{COMPONENT_HOST, STRUCTURED_CODE, STYLE_HOST};
pub use language::{
    CommentSyntax, LanguageProfile, LanguageRegistration, LanguageRole, comment_syntax,
    comment_syntax_for_extension, detect_language, language_profile,
    language_profile_for_extension, language_profiles,
};
pub use languages::css::PROFILE as CSS_LANGUAGE;
pub use languages::javascript::PROFILE as JAVASCRIPT_LANGUAGE;
pub use languages::less::PROFILE as LESS_LANGUAGE;
pub use languages::rust::PROFILE as RUST_LANGUAGE;
pub use languages::scss::PROFILE as SCSS_LANGUAGE;
pub use languages::shell::PROFILE as SHELL_LANGUAGE;
pub use languages::typescript::PROFILE as TYPESCRIPT_LANGUAGE;
pub use tool::{
    ArgumentPattern, CiWorkload, CommandPattern, TaskKind, TestRetryConfiguration,
    TestRetryProfile, TestRetrySignal, ToolId, ToolProfile, ToolRegistration, classify_tool,
    normalize_invocation, tool_profile, tool_profiles,
};
pub use tools::documentation::{CODESPELL, VALE};
pub use tools::stylesheet::STYLELINT;
pub use traversal::{TraversalDirectory, TraversalDirectoryRegistration, traversal_directories};
pub use verbosity::{
    LanguageVerbosity, VERBOSITY_BASELINE, VERBOSITY_CORPUS, VERBOSITY_CORPUS_REVISION,
    VerbosityRatio, verbosity, verbosity_ratio, verbosity_ratios,
};

pub use convention::{
    LanguageConventions, TestLayoutDefaults, TypecheckConvention, language_conventions,
};
pub use registry_inventory as registry;
