// Tests for `src/codebase/profiles/artifact.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::artifact_profiles;

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
