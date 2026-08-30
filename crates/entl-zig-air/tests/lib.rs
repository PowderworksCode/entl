// Tests for `src/lib.rs`: the two modules the binary is a front over.
//
// They are public so the reader can be examined without running a compiler, and
// so the store can be written to a directory a test chose. Reaching both is the
// test: either being made private breaks the tests beside them, not this file.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn the_reader_is_reachable_and_starts_empty() {
    let mut reader = entl_zig_air::air::Reader::default();
    assert!(
        reader.finish().is_none(),
        "an empty reader yields no function"
    );
}

#[test]
fn the_store_module_is_public() {
    let directory = tempfile::tempdir().unwrap();
    let store = entl_zig_air::store::Store::create(directory.path());
    assert!(store.is_ok(), "a store can be created in a fresh directory");
}
