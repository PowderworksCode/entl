// Tests for `src/codebase/profiles/mod.rs`.
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
fn built_in_profiles_are_complete_and_deterministic() {
    let languages = language_profiles()
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let ecosystems = ecosystem_profiles()
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();

    assert!(languages.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(languages.contains(&"rust"));
    assert!(languages.contains(&"javascript"));
    assert!(languages.contains(&"typescript"));
    assert_eq!(ecosystems, ["bun", "cargo", "npm", "pnpm", "yarn"]);
}
