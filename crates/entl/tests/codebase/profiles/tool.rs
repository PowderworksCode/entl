// Tests for `src/codebase/profiles/tool.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{CODESPELL, CiWorkload, STYLELINT, VALE, tool_profile, tool_profiles};
use std::collections::BTreeSet;

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
