// Tests for `src/codebase/profiles/artifacts.rs`.
//
// These four statics are what a discovered build output is matched against, so
// each has to be registered under the id it declares.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{
    BINARY_ARTIFACT, NAPI_ARTIFACT, SITE_ARTIFACT, TAURI_ARTIFACT, artifact_profiles,
};

#[test]
fn every_declared_artifact_profile_is_registered() {
    let registered: Vec<&str> = artifact_profiles()
        .iter()
        .map(|profile| profile.id)
        .collect();
    for profile in [
        &BINARY_ARTIFACT,
        &NAPI_ARTIFACT,
        &SITE_ARTIFACT,
        &TAURI_ARTIFACT,
    ] {
        assert!(
            registered.contains(&profile.id),
            "{} is declared but not registered",
            profile.id
        );
    }
}
