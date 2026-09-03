// Tests for `src/catalog.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::fs;

use tempfile::tempdir;

use entl_tree_sitter::*;

use sha2::{Digest, Sha256};

/// Computed here rather than borrowed from the crate: a manifest digest the
/// test derives with the same function the loader uses would agree with it even
/// when both are wrong.
fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn loads_a_verified_pack() {
    let directory = tempdir().unwrap();
    let grammar = b"not a real grammar";
    fs::write(directory.path().join("grammar.wasm"), grammar).unwrap();
    fs::write(
        directory.path().join(MANIFEST_FILENAME),
        format!(
            "schema = 1\nid = \"rust-test\"\nlanguage = \"rust\"\nversion = \"1\"\nsource = \"https://example.com/rust\"\nrevision = \"abc\"\nlicense = \"MIT\"\nabi = 15\nsha256 = \"{}\"\ncomparison-domain = \"rust\"\n",
            hex_digest(grammar)
        ),
    )
    .unwrap();

    let pack = ParserPack::load(directory.path()).unwrap();
    assert_eq!(pack.language().id, "rust");
    assert_eq!(pack.grammar(), grammar);
}

#[test]
fn rejects_unverified_bytes() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("grammar.wasm"), b"changed").unwrap();
    fs::write(
        directory.path().join(MANIFEST_FILENAME),
        "schema = 1\nid = \"rust-test\"\nlanguage = \"rust\"\nversion = \"1\"\nsource = \"https://example.com/rust\"\nrevision = \"abc\"\nlicense = \"MIT\"\nabi = 15\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\ncomparison-domain = \"rust\"\n",
    )
    .unwrap();

    assert!(matches!(
        ParserPack::load(directory.path()),
        Err(Error::DigestMismatch { .. })
    ));
}
