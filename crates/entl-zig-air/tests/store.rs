// Tests for `src/store.rs`: writing what was read, as Parquet.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn a_store_creates_its_directory_and_refuses_a_file() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("out");
    assert!(entl_zig_air::store::Store::create(&directory).is_ok());

    let file = temp.path().join("not-a-directory");
    std::fs::write(&file, b"").unwrap();
    assert!(
        entl_zig_air::store::Store::create(&file).is_err(),
        "a path that is a file cannot become an output directory"
    );
}
