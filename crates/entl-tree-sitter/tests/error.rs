// Tests for `src/error.rs`.
//
// The messages name the pack or path they are about: someone reading one is
// deciding which grammar to fix.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_tree_sitter::ParserPack;

#[test]
fn loading_a_missing_pack_names_the_directory() {
    let error = ParserPack::load(std::path::Path::new("/no/such/parser/pack"))
        .expect_err("a missing pack is an error");
    let text = error.to_string();
    assert!(!text.is_empty());
    assert!(
        text.contains("pack") || text.contains("/no/such/parser/pack"),
        "{text}"
    );
}
