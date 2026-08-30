// Tests for `src/codebase/profiles/convention.rs`.
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
fn language_conventions_are_colocated_with_profiles() {
    assert_eq!(
        language_profiles()
            .iter()
            .filter(|language| language.conventions.is_some())
            .map(|language| language.id)
            .collect::<Vec<_>>(),
        ["javascript", "rust", "typescript"]
    );
    let rust = language_conventions(&RUST_LANGUAGE).unwrap();
    assert_eq!(
        rust.inline_test_indicator("#[cfg(test)]\nmod tests {}\n"),
        Some("#[cfg(test)]")
    );
    let typescript = language_conventions(&TYPESCRIPT_LANGUAGE).unwrap();
    assert_eq!(
        typescript.typecheck.unwrap().config_files,
        ["tsconfig.json"]
    );
    assert_eq!(
        language_conventions(&JAVASCRIPT_LANGUAGE)
            .unwrap()
            .typecheck
            .unwrap()
            .config_files,
        ["jsconfig.json", "tsconfig.json"]
    );
    assert_eq!(typescript.test_layout.source_roots, ["src"]);
    assert_eq!(typescript.test_layout.test_root, "tests");
}
