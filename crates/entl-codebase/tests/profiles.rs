#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::collections::BTreeSet;
use std::path::Path;

use entl_codebase::{
    CODESPELL, COMPONENT_HOST, CiWorkload, EcosystemRole, JAVASCRIPT_LANGUAGE, ManifestSelection,
    RUST_LANGUAGE, SHELL_LANGUAGE, STRUCTURED_CODE, STYLE_HOST, STYLELINT, TYPESCRIPT_LANGUAGE,
    VALE, artifact_profiles, ecosystem_profile, ecosystem_profiles, language_conventions,
    language_facet, language_facets, language_profile, language_profiles, tool_profile,
    tool_profiles, traversal_directories,
};

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

#[test]
fn tool_profiles_are_codebase_owned_and_reference_typed_languages() {
    let profiles = tool_profiles();
    assert!(profiles.windows(2).all(|pair| pair[0].id < pair[1].id));
    assert_eq!(
        profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<BTreeSet<_>>()
            .len(),
        profiles.len()
    );
    assert!(
        profiles
            .iter()
            .flat_map(|profile| profile.languages)
            .all(|language| !language.id.is_empty())
    );
    assert!(std::ptr::eq(tool_profile("codespell").unwrap(), &CODESPELL));
    assert!(std::ptr::eq(tool_profile("vale").unwrap(), &VALE));
    assert!(std::ptr::eq(tool_profile("stylelint").unwrap(), &STYLELINT));
    assert!(STYLELINT.configuration_files.contains(&".stylelintrc.json"));
    assert_eq!(
        tool_profile("cargo").unwrap().ci_workload,
        CiWorkload::Heavy
    );
    assert!(tool_profile("cargo").unwrap().test_retry.is_some());
}

#[test]
fn artifact_profiles_are_registered_codebase_facts() {
    assert_eq!(
        artifact_profiles()
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>(),
        ["binary", "napi", "site", "tauri"]
    );
}

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

#[test]
fn language_profiles_colocate_detection_and_syntax() {
    let rust = language_profile("rust").unwrap();
    assert!(std::ptr::eq(rust, &RUST_LANGUAGE));
    assert!(rust.detects_source(Path::new("src/lib.rs")));
    assert_eq!(rust.comments.unwrap().line, ["//"]);

    let typescript = language_profile("typescript").unwrap();
    let javascript = language_profile("javascript").unwrap();
    let shell = language_profile("shell").unwrap();
    assert!(std::ptr::eq(typescript, &TYPESCRIPT_LANGUAGE));
    assert!(std::ptr::eq(javascript, &JAVASCRIPT_LANGUAGE));
    assert!(std::ptr::eq(shell, &SHELL_LANGUAGE));
    assert!(typescript.supersedes(javascript));
    assert!(typescript.accepts_source(Path::new("src/legacy.js")));
    assert!(typescript.has_facet(&STRUCTURED_CODE));
    assert!(typescript.has_facet(&STYLE_HOST));
    assert!(typescript.has_facet(&COMPONENT_HOST));
    assert!(rust.has_facet(&STRUCTURED_CODE));
    assert!(!rust.has_facet(&STYLE_HOST));
    assert!(std::ptr::eq(
        language_facet("style-host").unwrap(),
        &STYLE_HOST
    ));
    assert_eq!(
        language_facets()
            .iter()
            .map(|facet| facet.id)
            .collect::<Vec<_>>(),
        ["component-host", "structured-code", "style-host"]
    );
}

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
