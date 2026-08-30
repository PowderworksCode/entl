// Tests for `src/codebase/model/file.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn content_reads_are_lazy_and_codebase_relative() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "src/lib.rs", "pub const VALUE: u8 = 7;\n");
    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert_eq!(
        inventory.read_text("src/lib.rs").unwrap(),
        "pub const VALUE: u8 = 7;\n"
    );
    assert!(inventory.read_text("../outside").is_err());
    assert!(inventory.read_text(temp.path().join("src/lib.rs")).is_err());
}
