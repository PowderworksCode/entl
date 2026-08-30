// Tests for `src/codebase/model/diagnostic.rs`.
//
// A diagnostic is how the walk reports something it could not make sense of
// without failing the whole observation. It carries the path it is about,
// because a diagnostic nobody can locate is a rumour.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn a_malformed_manifest_is_reported_against_its_own_path() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "Cargo.toml", "this is not toml {{{\n");
    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let diagnostic = inventory
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.path.ends_with("Cargo.toml"))
        .expect("the malformed manifest is reported");
    assert!(!diagnostic.message.is_empty());
}
