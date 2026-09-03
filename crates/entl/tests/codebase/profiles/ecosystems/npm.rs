// Tests for `src/codebase/profiles/ecosystems/npm.rs`.
//
// The manifest and lockfile names are how a package manager is recognised
// on disk. A wrong name here does not fail: the ecosystem is simply never
// found, and every fact that depends on it goes quietly missing.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::ecosystem_profile;

#[test]
fn npm_is_registered_with_its_manifest_and_lockfiles() {
    let profile = ecosystem_profile("npm").expect("npm is registered");
    assert_eq!(profile.id, "npm");
    assert_eq!(profile.manifest, Some("package.json"));
    assert_eq!(
        profile.lockfiles,
        ["package-lock.json", "npm-shrinkwrap.json"]
    );
}
