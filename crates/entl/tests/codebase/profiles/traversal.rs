// Tests for `src/codebase/profiles/traversal.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::traversal_directories;

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
