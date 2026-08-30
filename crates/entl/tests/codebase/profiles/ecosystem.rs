// Tests for `src/codebase/profiles/ecosystem.rs`.
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
fn ecosystems_have_roles_and_zero_or_more_language_implications() {
    let bun = ecosystem_profile("bun").unwrap();
    assert!(bun.has_role(EcosystemRole::PackageManager));
    assert!(bun.has_role(EcosystemRole::Runtime));
    assert!(bun.implies_language(language_profile("javascript").unwrap()));

    let manifests = ecosystem_profiles()
        .iter()
        .filter_map(|profile| profile.manifest)
        .collect::<BTreeSet<_>>();
    for manifest in manifests {
        assert_eq!(
            ecosystem_profiles()
                .iter()
                .filter(|profile| {
                    profile.manifest == Some(manifest)
                        && matches!(profile.manifest_selection, ManifestSelection::Default)
                })
                .count(),
            1
        );
    }
}

#[test]
fn ecosystems_colocate_gitignore_conventions() {
    assert_eq!(
        ecosystem_profile("cargo").unwrap().gitignore_patterns,
        ["target/"]
    );
    for ecosystem in ["bun", "npm", "pnpm", "yarn"] {
        assert_eq!(
            ecosystem_profile(ecosystem).unwrap().gitignore_patterns,
            ["node_modules/"]
        );
    }
}
