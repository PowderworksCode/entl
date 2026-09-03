// Tests for `src/codebase/model/codebase.rs`.
//
// The tree is what every consumer asks about a file, and it answers by path
// relative to the codebase root. That relativity is the contract: an absolute
// path from the caller's machine would find nothing.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn files_are_found_by_their_codebase_relative_path() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
    );
    write(
        temp.path(),
        "src/lib.rs",
        "pub fn value() -> bool { true }\n",
    );
    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();

    assert!(inventory.has_file("src/lib.rs"));
    assert!(inventory.file("src/lib.rs").is_some());
    assert!(!inventory.has_file("src/missing.rs"));
    assert!(!inventory.has_file(temp.path().join("src/lib.rs")));
}

#[test]
fn files_can_be_selected_by_language() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
    );
    write(
        temp.path(),
        "src/lib.rs",
        "pub fn value() -> bool { true }\n",
    );
    write(temp.path(), "README.md", "# Fixture\n");
    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();

    let rust: Vec<_> = inventory
        .files_with_language("rust")
        .map(|entry| entry.path.clone())
        .collect();
    assert_eq!(rust, [std::path::PathBuf::from("src/lib.rs")]);
}
