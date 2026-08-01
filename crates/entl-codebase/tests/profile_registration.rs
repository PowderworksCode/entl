#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_codebase::{
    EcosystemProfile, EcosystemRegistration, EcosystemRole, LanguageConventions, LanguageFacet,
    LanguageFacetRegistration, LanguageProfile, LanguageRegistration, LanguageRole,
    ManifestSelection, TestLayoutDefaults, ecosystem_profile, language_conventions, language_facet,
    language_profile, profile_registry,
};

static FIXTURE_FACET: LanguageFacet = LanguageFacet {
    id: "fixture-source-surface",
    description: "fixture language source surface",
};

static FIXTURE_LANGUAGE: LanguageProfile = LanguageProfile {
    id: "fixture-language",
    display_name: "Fixture Language",
    extensions: &["fixture"],
    source_extensions: &["fixture"],
    filenames: &[],
    shebangs: &[],
    role: LanguageRole::Programming,
    facets: &[&FIXTURE_FACET],
    comments: None,
    conventions: Some(LanguageConventions {
        typecheck: None,
        test_layout: TestLayoutDefaults {
            source_roots: &["source"],
            test_root: "checks",
            test_suffixes: &[".check"],
        },
        inline_test_detector: |_| None,
    }),
    config_files: &[],
    package_dependencies: &[],
    supersedes: &[],
};

profile_registry::submit! { LanguageFacetRegistration(&FIXTURE_FACET) }
profile_registry::submit! { LanguageRegistration(&FIXTURE_LANGUAGE) }

static POLYGLOT: EcosystemProfile = EcosystemProfile {
    id: "fixture-polyglot",
    display_name: "Fixture Polyglot Build",
    roles: &[EcosystemRole::BuildSystem],
    implied_languages: &[],
    manifest: None,
    lockfiles: &[],
    selector_files: &[],
    gitignore_patterns: &[],
    manifest_selection: ManifestSelection::Default,
    dependency_pins: None,
};

profile_registry::submit! {
    EcosystemRegistration(&POLYGLOT)
}

#[test]
fn downstream_polyglot_ecosystems_need_not_belong_to_a_language() {
    let profile = ecosystem_profile("fixture-polyglot").unwrap();
    assert!(profile.implied_languages.is_empty());
    assert!(profile.has_role(EcosystemRole::BuildSystem));
}

#[test]
fn downstream_languages_link_registered_facets_by_identity() {
    let facet = language_facet("fixture-source-surface").unwrap();
    let language = language_profile("fixture-language").unwrap();
    assert!(std::ptr::eq(facet, &FIXTURE_FACET));
    assert!(language.has_facet(facet));
    assert_eq!(
        language_conventions(language)
            .unwrap()
            .test_layout
            .test_root,
        "checks"
    );
}
