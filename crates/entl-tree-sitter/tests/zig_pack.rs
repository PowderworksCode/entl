#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The Zig pack, and the rewrite that keeps a grammar hole from deleting a file.
//!
//! `tree-sitter-zig` 1.1.2 cannot read an `if` expression in type position, and
//! rejects the whole file when it meets one. Every source below is a real shape
//! from Bun v1.3.14.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{LoadedParser, ParsedFile, ParserPack, ParserRuntime};
use tree_sitter::Node;

fn parser() -> LoadedParser {
    let pack_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("parser-packs/zig");
    let pack = Arc::new(ParserPack::load(pack_path).unwrap());
    ParserRuntime::new().unwrap().load(pack).unwrap()
}

fn parse(parser: &LoadedParser, source: &str) -> ParsedFile {
    parser
        .parse("src/main.zig", Arc::<[u8]>::from(source.as_bytes()))
        .unwrap()
}

/// Top-level declaration names, which is what a grammar hole costs when it
/// swallows one.
fn declarations(parsed: &ParsedFile) -> Vec<String> {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .filter(|node: &Node| {
            matches!(
                node.kind(),
                "function_declaration" | "test_declaration" | "variable_declaration"
            )
        })
        .filter_map(|node| {
            let mut inner = node.walk();
            node.named_children(&mut inner)
                .find(|child| child.kind() == "identifier")
                .and_then(|name| {
                    std::str::from_utf8(&parsed.source[name.start_byte()..name.end_byte()]).ok()
                })
                .map(str::to_owned)
        })
        .collect()
}

#[test]
fn loads_and_parses_with_the_zig_wasm_pack() {
    let parsed = parse(&parser(), "pub fn answer() u32 { return 42; }");
    assert!(!parsed.tree.root_node().has_error());
    assert_eq!(parsed.provenance.parser_id, "tree-sitter-zig");
    assert_eq!(parsed.provenance.parser_version, "1.1.2");
    assert!(parsed.rewrites.is_empty(), "nothing needed rewriting");
    assert!(!parsed.rewrites_narrowed);
}

#[test]
fn a_conditional_return_type_does_not_delete_the_file_around_it() {
    // The shape from cli/pack_command.zig. Without the rewrite the ERROR node
    // spans `write`, and `keep` and `after` go with it.
    let source = "\
pub const keep = 1;
pub fn write(comptime publish: bool) Error!if (publish) Context(true) else void {
    return;
}
pub const after = 2;
";
    let parser = parser();

    let parsed = parse(&parser, source);
    assert!(
        !parsed.tree.root_node().has_error(),
        "the rewrite should have produced a clean parse"
    );
    assert_eq!(declarations(&parsed), ["keep", "write", "after"]);

    // The rewrite is reported, and reported as having narrowed the source:
    // `write` now returns `void` unconditionally, which is not what the file
    // says.
    assert_eq!(parsed.rewrites.len(), 1);
    assert!(
        parsed.rewrites[0].contains("type position"),
        "{:?}",
        parsed.rewrites
    );
    assert!(
        parsed.rewrites_narrowed,
        "a discarded branch is not a faithful signature"
    );
}

#[test]
fn a_rewrite_moves_no_byte_a_consumer_could_report() {
    let source = "\
pub const first = 1;
pub fn f(comptime w: bool) []if (w) u16 else u8 {
    return &.{};
}
pub const last = 2;
";
    let parsed = parse(&parser(), source);
    assert!(!parsed.tree.root_node().has_error());
    assert_eq!(parsed.source.len(), source.len());

    // `last` is after the rewrite, so its span is the one that would move.
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let last = root
        .named_children(&mut cursor)
        .filter(|node| node.kind() == "variable_declaration")
        .last()
        .expect("the trailing declaration");
    assert_eq!(
        last.start_byte(),
        source.find("pub const last").expect("present")
    );
}

#[test]
fn source_the_grammar_accepts_is_never_rewritten() {
    // An `if` in VALUE position is ordinary Zig and parses, so the rewrite must
    // not run at all -- the file never fails, so it is never retried.
    let source = "\
pub fn f(w: bool) u8 {
    const x = if (w) @as(u8, 1) else @as(u8, 2);
    return x;
}
";
    let parsed = parse(&parser(), source);
    assert!(!parsed.tree.root_node().has_error());
    assert!(parsed.rewrites.is_empty());
    assert_eq!(parsed.source.as_ref(), source.as_bytes());
}
