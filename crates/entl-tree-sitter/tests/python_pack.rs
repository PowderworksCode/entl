#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The Python pack, and what a grammar hole costs in a newline-delimited
//! language.
//!
//! Measured with `--example parse-rate` before any of this was written:
//!
//! ```text
//!                        installed Python   CPython main @ 2ba0b2c9d1d0
//!   files                           5,676                        2,346
//!   rejected, no rewrites               0                           61
//!   rejected, with rewrites             0                            5
//! ```
//!
//! The construct behind 56 of those 61 is PEP 810's `lazy import`, which the
//! rewrite table blanks. The five that remain are all in CPython's own test
//! suite and are covered at the bottom of this file, because a gap that is
//! known and declined is not the same as one nobody looked for.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{LoadedParser, ParsedFile, ParserPack, ParserRuntime};

fn parser() -> LoadedParser {
    let pack_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("parser-packs/python");
    let pack = Arc::new(ParserPack::load(pack_path).unwrap());
    ParserRuntime::new().unwrap().load(pack).unwrap()
}

fn parse(parser: &LoadedParser, source: &str) -> ParsedFile {
    parser
        .parse("module.py", Arc::<[u8]>::from(source.as_bytes()))
        .unwrap()
}

/// Named top-level statements, which is what a consumer reads a module for.
fn statements(parsed: &ParsedFile) -> Vec<String> {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .map(|node| node.kind().to_owned())
        .collect()
}

#[test]
fn loads_and_parses_with_the_python_wasm_pack() {
    let parsed = parse(&parser(), "def answer() -> int:\n    return 42\n");
    assert!(!parsed.tree.root_node().has_error());
    assert_eq!(parsed.provenance.parser_id, "tree-sitter-python");
    assert_eq!(parsed.provenance.parser_version, "0.25.0");
    assert!(parsed.rewrites.is_empty(), "nothing needed rewriting");
    assert!(!parsed.rewrites_narrowed);
}

/// The constructs the grammar was most likely to be behind on. Every one of
/// these was checked against the corpus rather than assumed, because a grammar
/// that cannot read them would have made the pack useless for modern Python.
#[test]
fn modern_python_needs_no_rewriting_at_all() {
    let parser = parser();
    for source in [
        // PEP 634 structural pattern matching
        "match command.split():\n    case [action, obj]:\n        pass\n    case _:\n        pass\n",
        // PEP 572 walrus
        "if (n := len(a)) > 10:\n    pass\n",
        // PEP 570 positional-only and keyword-only parameters
        "def f(a, b, /, c, *, d):\n    pass\n",
        // PEP 695 type parameters and type aliases
        "type Alias[T] = list[T]\ndef first[T](items: list[T]) -> T:\n    return items[0]\n",
        "class Box[T]:\n    def get(self) -> T: ...\n",
        // PEP 530 async comprehensions
        "async def f():\n    return [x async for x in aiter() if await check(x)]\n",
        // PEP 701 f-strings that reuse the outer quote
        "x = f\"{d[\"key\"]}\"\n",
        // PEP 646 variadic generics
        "def f(*args: *Ts) -> tuple[*Ts]: ...\n",
        // PEP 604 unions and PEP 655 qualifiers
        "def f(x: int | None = None) -> str | bytes: ...\n",
        // decorators that are arbitrary expressions (PEP 614)
        "@buttons[0].clicked.connect\ndef handler(): ...\n",
        // exception groups (PEP 654)
        "try:\n    pass\nexcept* TypeError:\n    pass\n",
    ] {
        let parsed = parse(&parser, source);
        assert!(
            !parsed.tree.root_node().has_error(),
            "the grammar should read this as written: {source}"
        );
        assert!(
            parsed.rewrites.is_empty(),
            "and without a rewrite: {source}"
        );
    }
}

// -- PEP 810 lazy imports ----------------------------------------------------

#[test]
fn a_lazy_import_does_not_cost_the_file_its_facts() {
    // Both spellings, from CPython `main`: `Lib/typing.py` has the first and
    // `Lib/concurrent/futures/__init__.py` the second.
    let source = "\
import os
lazy import json
lazy from collections import defaultdict


def keep():
    return 1
";
    let parsed = parse(&parser(), source);
    assert!(
        !parsed.tree.root_node().has_error(),
        "the rewrite should have produced a clean parse"
    );
    assert_eq!(
        statements(&parsed),
        [
            "import_statement",
            "import_statement",
            "import_from_statement",
            "function_definition",
        ]
    );
    assert_eq!(parsed.rewrites.len(), 2, "{:?}", parsed.rewrites);
    assert!(
        parsed.rewrites[0].contains("PEP 810"),
        "{:?}",
        parsed.rewrites
    );
}

/// Blanking `lazy` leaves the module imported and the name bound; only the
/// moment the import runs changes, and no fact Entl reports names that. This is
/// the same standard by which blanking `const` out of `const trait` counts as
/// faithful.
#[test]
fn a_lazy_import_rewrite_does_not_narrow_what_the_module_says() {
    let parsed = parse(&parser(), "lazy import os\n");
    assert!(!parsed.rewrites.is_empty());
    assert!(!parsed.rewrites_narrowed);
}

#[test]
fn a_rewrite_moves_no_byte_a_consumer_could_report() {
    let source = "\
lazy import json


def last():
    return 2
";
    let parsed = parse(&parser(), source);
    assert!(!parsed.tree.root_node().has_error());
    assert_eq!(parsed.source.len(), source.len());

    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let last = root
        .named_children(&mut cursor)
        .find(|node| node.kind() == "function_definition")
        .expect("the trailing definition");
    assert_eq!(last.start_byte(), source.find("def last").expect("present"));
}

/// `lazy` is a SOFT keyword, so position is the only thing that separates PEP
/// 810 from a module that happens to be called `lazy`. CPython's own
/// `Lib/test/test_syntax.py` holds both, which is why this is anchored to the
/// start of a line rather than to the word.
#[test]
fn a_module_named_lazy_is_not_mistaken_for_a_lazy_import() {
    let parser = parser();
    for source in [
        "from .lazy import x\n",
        "from ...lazy import x\n",
        "from . sub.lazy import x\n",
        "lazy = 1\nimport os\n",
        "lazy: int = 1\n",
        "print(lazy, imports)\n",
    ] {
        let parsed = parse(&parser, source);
        assert!(
            parsed.rewrites.is_empty(),
            "`lazy` is an ordinary identifier here: {source}"
        );
        assert_eq!(parsed.source.as_ref(), source.as_bytes());
    }
}

#[test]
fn source_the_grammar_accepts_is_never_rewritten() {
    let source = "import os\nfrom collections import defaultdict\n";
    let parsed = parse(&parser(), source);
    assert!(!parsed.tree.root_node().has_error());
    assert!(parsed.rewrites.is_empty());
    assert_eq!(parsed.source.as_ref(), source.as_bytes());
}

// -- The gaps that were measured and left open -------------------------------

/// Four constructs `tree-sitter-python` 0.25.0 cannot read.
///
/// Each accounts for exactly one file in CPython `main` and all four are in its
/// test suite, which is adversarial by design; against 5,676 files of installed
/// Python not one of them occurs. They are recorded here rather than rewritten
/// so that the next person measures the corpus instead of the grammar.
///
/// PEP 696 is the one to watch: type-parameter defaults are released syntax as
/// of 3.13, so unlike the other three this will spread into ordinary libraries.
#[test]
fn the_gaps_that_remain_are_known_and_still_gaps() {
    let parser = parser();
    for (source, why) in [
        (
            "def f[T = int](): pass\n",
            "PEP 696 type-parameter defaults -- Lib/test/test_type_params.py",
        ),
        (
            "x = Union[*(k.values())]\n",
            "a parenthesized starred expression in a subscript -- Lib/test/test_annotationlib.py",
        ),
        (
            "match x:\n    case +0:\n        pass\n",
            "a `+`-signed literal pattern -- Lib/test/test_patma.py",
        ),
        (
            "def f():\n    (bar.\nbaz)\n",
            "a parenthesized continuation dedented past its block -- Lib/test/test_compile.py",
        ),
    ] {
        let parsed = parse(&parser, source);
        assert!(
            parsed.tree.root_node().has_error(),
            "this gap has closed and the note above should be revisited: {why}"
        );
        assert!(
            parsed.rewrites.is_empty(),
            "nothing claims to rewrite this: {why}"
        );
    }
}
