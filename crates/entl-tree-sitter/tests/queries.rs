#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Loading, compiling, and running the queries a parser pack ships.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{Error, LoadedParser, ParsedFile, ParserPack, ParserRuntime};

fn rust_pack_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parser-packs/rust")
}

fn rust_parser() -> LoadedParser {
    let pack = Arc::new(ParserPack::load(rust_pack_path()).unwrap());
    ParserRuntime::new().unwrap().load(pack).unwrap()
}

fn parse(parser: &LoadedParser, source: &str) -> ParsedFile {
    parser
        .parse("source.rs", Arc::<[u8]>::from(source.as_bytes()))
        .unwrap()
}

/// Copy the rust pack somewhere writable so a test can add a query to it.
fn pack_with_query(name: &str, query: &str) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("rust");
    std::fs::create_dir_all(root.join("queries")).unwrap();
    for entry in std::fs::read_dir(rust_pack_path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
        }
    }
    std::fs::write(root.join("queries").join(format!("{name}.scm")), query).unwrap();
    (directory, root)
}

#[test]
fn loads_the_queries_a_pack_ships() {
    let pack = ParserPack::load(rust_pack_path()).unwrap();
    // vendored from the upstream grammar, alongside the wasm
    assert!(pack.queries().contains_key("highlights"), "{:?}", pack.queries().keys());
    assert!(!pack.queries()["highlights"].is_empty());
    assert_eq!(pack.queries_sha256().len(), 64);
}

/// A pack with no queries is ordinary, not an error.
#[test]
fn a_pack_without_queries_still_loads() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("rust");
    std::fs::create_dir_all(&root).unwrap();
    for entry in std::fs::read_dir(rust_pack_path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
        }
    }
    let pack = ParserPack::load(&root).unwrap();
    assert!(pack.queries().is_empty());
    assert!(ParserRuntime::new().unwrap().load(Arc::new(pack)).is_ok());
}

/// The point of compiling at load: a broken query cannot reach a consumer.
///
/// A query that does not compile would otherwise match nothing, and a rule that
/// matches nothing reports nothing, which reads exactly like a clean file.
#[test]
fn a_query_that_does_not_compile_fails_the_load() {
    let (_directory, root) = pack_with_query("discards", "(no_such_node_kind) @x");
    let pack = Arc::new(ParserPack::load(&root).unwrap());
    let error = ParserRuntime::new().unwrap().load(pack).unwrap_err();
    assert!(
        matches!(&error, Error::CompileQuery { query, .. } if query == "discards"),
        "{error:?}"
    );
    let text = error.to_string();
    assert!(text.contains("discards") && text.contains("row"), "{text}");
}

/// Naming a query that is not there is an error, not an empty result.
#[test]
fn an_unknown_query_is_reported() {
    let parser = rust_parser();
    let file = parse(&parser, "fn a() {}");
    let error = parser.matches("nope", &file).unwrap_err();
    assert!(matches!(&error, Error::UnknownQuery { query, .. } if query == "nope"));
    assert!(error.to_string().contains("highlights"), "{error}");
}

/// The forms the discard analyzer needs, run through the pack machinery.
#[test]
fn runs_a_query_and_returns_named_captures() {
    // an anonymous node, which is how a wildcard binding is spelled
    let (_directory, root) = pack_with_query(
        "discards",
        r#"(let_declaration pattern: "_" value: (call_expression)) @discard.let-underscore"#,
    );
    let pack = Arc::new(ParserPack::load(&root).unwrap());
    let parser = ParserRuntime::new().unwrap().load(pack).unwrap();
    let file = parse(&parser, "fn a() { let _ = w(); }\nfn b() { let x = w(); }");
    let found = parser.matches("discards", &file).unwrap();
    assert_eq!(found.len(), 1, "only the wildcard binding: {found:?}");
    assert!(found[0].has("discard.let-underscore"));
}

/// Absence of an optional capture is how a query says "no binding here".
///
/// Tree-sitter queries have no negation, and half the discard forms turn on
/// exactly that distinction, so this is the load-bearing behavior.
#[test]
fn an_optional_capture_is_absent_when_it_does_not_match() {
    let (_directory, root) = pack_with_query(
        "discards",
        r#"((match_arm pattern: (match_pattern
             (tuple_struct_pattern type: (identifier) @variant (identifier)? @bind))) @arm
           (#eq? @variant "Err"))"#,
    );
    let pack = Arc::new(ParserPack::load(&root).unwrap());
    let parser = ParserRuntime::new().unwrap().load(pack).unwrap();
    let file = parse(
        &parser,
        "fn a() { match w() { Ok(v) => v, Err(_) => return } }\n\
         fn b() { match w() { Ok(v) => v, Err(e) => report(e) } }",
    );
    let found = parser.matches("discards", &file).unwrap();
    assert_eq!(found.len(), 2, "both Err arms: {found:?}");

    let discarded = found.iter().filter(|found| !found.has("bind")).count();
    let bound = found.iter().filter(|found| found.has("bind")).count();
    assert_eq!((discarded, bound), (1, 1), "Err(_) drops, Err(e) binds");
}

/// A predicate narrows a match to one method name.
#[test]
fn a_predicate_filters_on_matched_text() {
    let (_directory, root) = pack_with_query(
        "discards",
        r#"((call_expression function: (field_expression field: (field_identifier) @method)) @site
           (#eq? @method "ok"))"#,
    );
    let pack = Arc::new(ParserPack::load(&root).unwrap());
    let parser = ParserRuntime::new().unwrap().load(pack).unwrap();
    let file = parse(&parser, "fn a() { w().ok(); }\nfn b() { w().unwrap(); }");
    let found = parser.matches("discards", &file).unwrap();
    assert_eq!(found.len(), 1, "only `.ok()`: {found:?}");
    assert!(found[0].capture("site").is_some());
}

/// Provenance has to separate two runs whose queries differ.
///
/// Recording only the grammar digest cannot: the grammar is identical in both
/// of these, and the queries are what produced the difference in what matched.
#[test]
fn provenance_records_the_query_digest() {
    let (_first_dir, first_root) = pack_with_query("discards", "(identifier) @a");
    let (_second_dir, second_root) = pack_with_query("discards", "(field_identifier) @b");
    let (_same_dir, same_root) = pack_with_query("discards", "(identifier) @a");

    let provenance = |root: &PathBuf| {
        let pack = Arc::new(ParserPack::load(root).unwrap());
        let parser = ParserRuntime::new().unwrap().load(pack).unwrap();
        parse(&parser, "fn a() {}").provenance
    };
    let first = provenance(&first_root);
    let second = provenance(&second_root);
    let same = provenance(&same_root);

    assert_eq!(first.queries_sha256.len(), 64);
    assert_eq!(
        first.grammar_sha256, second.grammar_sha256,
        "same grammar, so only the queries can distinguish them"
    );
    assert_ne!(
        first.queries_sha256, second.queries_sha256,
        "different queries must not carry identical provenance"
    );
    assert_eq!(
        first.queries_sha256, same.queries_sha256,
        "the digest has to be stable for identical queries"
    );
}

/// A pack shipping no queries still has a stable digest, not an empty one.
#[test]
fn a_pack_without_queries_has_a_stable_digest() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("rust");
    std::fs::create_dir_all(&root).unwrap();
    for entry in std::fs::read_dir(rust_pack_path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
        }
    }
    let pack = ParserPack::load(&root).unwrap();
    assert_eq!(pack.queries_sha256().len(), 64);
    assert_ne!(pack.queries_sha256(), ParserPack::load(rust_pack_path()).unwrap().queries_sha256());
}
