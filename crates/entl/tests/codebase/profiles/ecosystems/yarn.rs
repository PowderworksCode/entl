// Tests for `src/codebase/profiles/ecosystems/yarn.rs`.
//
// The manifest and lockfile names are how a package manager is recognised
// on disk. A wrong name here does not fail: the ecosystem is simply never
// found, and every fact that depends on it goes quietly missing.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::ecosystem_profile;

#[test]
fn yarn_is_registered_with_its_manifest_and_lockfiles() {
    let profile = ecosystem_profile("yarn").expect("yarn is registered");
    assert_eq!(profile.id, "yarn");
    assert_eq!(profile.manifest, Some("package.json"));
    assert_eq!(profile.lockfiles, ["yarn.lock"]);
}
