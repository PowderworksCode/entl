// Tests for `src/codebase/profiles/languages/scala.rs`.
//
// A profile is a declaration nothing else checks: it is read by id, and a
// typo in an extension makes the language invisible rather than wrong. Its
// registration and what it claims to match are the contract.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::language_profile;

#[test]
fn scala_is_registered_under_its_id() {
    let profile = language_profile("scala").expect("scala is registered");
    assert_eq!(profile.id, "scala");
    assert!(!profile.display_name.is_empty());
}

#[test]
fn scala_claims_its_extensions() {
    let profile = language_profile("scala").expect("scala is registered");
    assert_eq!(profile.extensions, ["scala"]);
}
