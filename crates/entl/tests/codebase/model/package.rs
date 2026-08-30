// Tests for `src/codebase/model/package.rs`.
//
// `as_str` is what puts a kind into Parquet and JSON, so its spellings are a
// wire format: renaming one is a change other tools see, not an internal edit.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::PackageKind;

#[test]
fn every_package_kind_has_a_stable_spelling() {
    for kind in [PackageKind::Cargo, PackageKind::Node] {
        let text = kind.as_str();
        assert!(!text.is_empty());
        assert_eq!(text, text.to_lowercase(), "{text} is not lowercase");
    }
    assert_ne!(PackageKind::Cargo.as_str(), PackageKind::Node.as_str());
}
