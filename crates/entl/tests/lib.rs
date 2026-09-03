// Tests for `src/lib.rs`: the crate's two public modules.
//
// entl exposes `codebase` and `github`, and every consumer — ordnung among them
// — binds to those two names. Making either private is a breaking change that
// surfaces somewhere else entirely.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn both_observation_modules_are_public() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("README.md"), "# Fixture\n").unwrap();
    let codebase =
        entl::codebase::inspect(temp.path(), &entl::codebase::InventoryOptions::default()).unwrap();
    entl::github::inspect(&codebase);
}
