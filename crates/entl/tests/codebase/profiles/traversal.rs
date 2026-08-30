// Tests for `src/codebase/profiles/traversal.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{
    CODESPELL, COMPONENT_HOST, CiWorkload, EcosystemRole, JAVASCRIPT_LANGUAGE, ManifestSelection,
    RUST_LANGUAGE, SHELL_LANGUAGE, STRUCTURED_CODE, STYLE_HOST, STYLELINT, TYPESCRIPT_LANGUAGE,
    VALE, artifact_profiles, ecosystem_profile, ecosystem_profiles, language_conventions,
    language_facet, language_facets, language_profile, language_profiles, tool_profile,
    tool_profiles, traversal_directories,
};
use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn traversal_conventions_are_registered_with_domain_evidence() {
    let build = traversal_directories()
        .iter()
        .find(|directory| directory.name == "build")
        .unwrap();
    assert_eq!(build.markers, ["package.json"]);
    let target = traversal_directories()
        .iter()
        .find(|directory| directory.name == "target")
        .unwrap();
    assert_eq!(target.markers, ["Cargo.toml"]);
}
