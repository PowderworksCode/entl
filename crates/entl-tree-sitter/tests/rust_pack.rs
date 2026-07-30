use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{ParserPack, ParserRuntime};

#[test]
fn loads_and_parses_with_the_rust_wasm_pack() {
    let pack_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("parser-packs/rust");
    let pack = Arc::new(ParserPack::load(pack_path).unwrap());
    let parser = ParserRuntime::new().unwrap().load(pack).unwrap();
    let parsed = parser
        .parse(
            "src/lib.rs",
            Arc::<[u8]>::from(&b"pub fn answer() -> u32 { 42 }"[..]),
        )
        .unwrap();

    assert!(!parsed.tree.root_node().has_error());
    assert_eq!(parsed.tree.root_node().kind(), "source_file");
    assert_eq!(parsed.provenance.parser_id, "tree-sitter-rust");
    assert_eq!(parsed.provenance.parser_version, "0.24.2");
}
