// Tests for `src/codebase/model/mod.rs`: the types the observation is made of.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{EcosystemId, LanguageId, PackageId, PackageKind, WorkspaceId, WorkspaceKind};

/// Naming them is the test: the model is what every consumer binds to, so a
/// type that stops being re-exported here breaks them rather than this crate.
#[test]
fn every_model_type_is_re_exported() {
    assert_eq!(LanguageId::from("rust").as_str(), "rust");
    assert_eq!(PackageId::from("cargo:.").as_str(), "cargo:.");
    assert_eq!(EcosystemId::from("cargo").as_str(), "cargo");
    assert_eq!(WorkspaceId::from("cargo:.").as_str(), "cargo:.");
    assert!(!PackageKind::Cargo.as_str().is_empty());
    assert!(!WorkspaceKind::Cargo.as_str().is_empty());
}
