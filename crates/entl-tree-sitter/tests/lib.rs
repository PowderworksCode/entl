// Tests for `src/lib.rs`: the crate's public surface.
//
// The re-exports are what entl and treebank bind to, so a name dropped here
// breaks them rather than this crate. Reaching each one is the test.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_tree_sitter::{Error, ParserPack, Result};

#[test]
fn the_surface_is_reachable() {
    let loaded: Result<ParserPack> = ParserPack::load(std::path::Path::new("/no/such/pack"));
    assert!(loaded.is_err());
    let _: Option<Error> = None;
}
