#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Per-language data a pack carries that is not expressible as a query.
//!
//! A Tree-sitter query can find a shape. It cannot say that `Result` means a
//! failure can be reported, that `binary_search`'s `Err` is an answer rather
//! than a failure, or that `.unwrap_or_default()` reads identically on a type
//! that never failed. Those are facts about a language's standard library, so
//! they travel with the pack that knows the language.

use std::path::PathBuf;

use entl_tree_sitter::{Error, ParserPack, ParserRuntime, Propagation};

fn typescript_pack_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parser-packs/typescript")
}

fn rust_pack_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parser-packs/rust")
}

/// Copy the rust pack somewhere writable, replacing its manifest.
fn pack_with_manifest(manifest: &str) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("rust");
    std::fs::create_dir_all(&root).unwrap();
    for entry in std::fs::read_dir(rust_pack_path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() && entry.file_name() != "parser.toml" {
            std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
        }
    }
    let original = std::fs::read_to_string(rust_pack_path().join("parser.toml")).unwrap();
    let head = original
        .split("\n[error-handling]")
        .next()
        .expect("the rust manifest still has an error-handling section");
    std::fs::write(root.join("parser.toml"), format!("{head}\n{manifest}")).unwrap();
    (directory, root)
}

/// The same, replacing the `[tokenization]` section rather than appending
/// after it.
fn pack_with_tokenization(tokenization: &str) -> (tempfile::TempDir, PathBuf) {
    let (directory, root) = pack_with_manifest("");
    let original = std::fs::read_to_string(root.join("parser.toml")).unwrap();
    let head = original
        .split("\n[tokenization]")
        .next()
        .expect("the rust manifest still has a tokenization section");
    std::fs::write(
        root.join("parser.toml"),
        format!("{head}\n[tokenization]\n{tokenization}"),
    )
    .unwrap();
    (directory, root)
}

#[test]
fn the_rust_pack_declares_how_rust_spells_failure() {
    let pack = ParserPack::load(rust_pack_path()).unwrap();
    let errors = &pack.manifest().error_handling;
    assert_eq!(errors.fallible_types, ["Result"]);
    assert_eq!(errors.optional_types, ["Option"]);
    assert!(
        errors
            .non_failure_results
            .iter()
            .any(|name| name == "binary_search"),
        "{:?}",
        errors.non_failure_results
    );
    assert!(
        errors
            .non_failure_results
            .iter()
            .any(|name| name == "strip_prefix"),
        "{:?}",
        errors.non_failure_results
    );
    // Both read identically on an `Option`, which never carried a failure:
    // `.unwrap_or_default()` yields the default either way, and `.unwrap()`
    // panics either way. Syntax cannot tell them apart.
    assert_eq!(errors.ambiguous_forms, ["unwrap-or", "panic"]);
    // The exclusion above applies only to the forms that identify a discard BY
    // the failure type. `.unwrap_or(..)` says nothing about what it unwrapped,
    // so excluding a `strip_prefix` receiver there would drop real findings.
    assert_eq!(
        errors.non_failure_results_forms,
        ["ok-discard", "ok-binding"]
    );
}

#[test]
fn the_rust_pack_declares_what_marks_a_test() {
    let pack = ParserPack::load(rust_pack_path()).unwrap();
    assert_eq!(pack.manifest().tests.markers, ["test"]);
    assert_eq!(pack.manifest().tests.module_markers, ["cfg(test)"]);
}

/// Packs written before this data existed must still load.
///
/// Every other pack in the tree omits both sections, and a language nobody has
/// characterized yet should say nothing rather than claim a default that is
/// wrong for it.
#[test]
fn a_pack_that_declares_neither_still_loads() {
    let (_directory, root) = pack_with_manifest("");
    let pack = ParserPack::load(root).unwrap();
    assert!(pack.manifest().error_handling.fallible_types.is_empty());
    assert!(pack.manifest().error_handling.ambiguous_forms.is_empty());
    assert!(pack.manifest().tests.markers.is_empty());
}

/// A misspelled key is an error, not silence.
///
/// These lists decide whether a finding is reported at all, so a pack that
/// meant to exclude `binary_search` and misspelled the key would produce a
/// confident wrong answer rather than a loud one.
#[test]
fn an_unknown_error_handling_key_is_rejected() {
    let (_directory, root) = pack_with_manifest("[error-handling]\nfallible-type = [\"Result\"]\n");
    let error = ParserPack::load(root).unwrap_err();
    assert!(
        matches!(error, Error::Manifest { .. }),
        "expected a manifest error, got {error}"
    );
    assert!(
        error.to_string().contains("fallible-type"),
        "the error should name the key: {error}"
    );
}

#[test]
fn an_unknown_tests_key_is_rejected() {
    let (_directory, root) = pack_with_manifest("[tests]\nmarker = [\"test\"]\n");
    let error = ParserPack::load(root).unwrap_err();
    assert!(
        matches!(error, Error::Manifest { .. }),
        "expected a manifest error, got {error}"
    );
}

/// Rust declares failure in the signature; TypeScript does not.
///
/// This decides what an unrecognized return type means. A Rust callable
/// returning neither `Result` nor `Option` genuinely cannot report a failure. A
/// TypeScript callable can always throw, so a signature that says nothing has
/// declined nothing, and calling it infallible would claim the error was
/// trapped when not catching it was available the whole time.
#[test]
fn propagation_distinguishes_declared_failure_from_unchecked() {
    let rust = ParserPack::load(rust_pack_path()).unwrap();
    assert_eq!(
        rust.manifest().error_handling.propagation,
        Propagation::Declared,
        "Rust declares failure in the return type"
    );

    let typescript = ParserPack::load(typescript_pack_path()).unwrap();
    assert_eq!(
        typescript.manifest().error_handling.propagation,
        Propagation::Unchecked,
        "any TypeScript callable can throw"
    );
}

/// The safe reading is the default: a pack that says nothing gets `Declared`,
/// which reports less reach rather than more.
#[test]
fn propagation_defaults_to_declared() {
    let (_directory, root) = pack_with_manifest("");
    let pack = ParserPack::load(root).unwrap();
    assert_eq!(
        pack.manifest().error_handling.propagation,
        Propagation::Declared
    );
}

/// A node kind the grammar does not define is the same failure as a misspelled
/// key, one step later: the pack loads, the kind matches nothing, and the pack
/// reads exactly as if it had declared nothing.
///
/// This is not hypothetical. The zig pack shipped `line_comment`,
/// `doc_comment`, `container_doc_comment` and `field_identifier`, and
/// `tree-sitter-zig` 1.1.2 has none of them — it spells them `comment` and
/// `identifier` — so every Zig comment was compared as if it were code and no
/// test could tell.
#[test]
fn a_node_kind_the_grammar_does_not_define_fails_the_load() {
    // `container_doc_comment` is a real kind in some grammars and not in this
    // one, which is the point: the manifest cannot be checked without the
    // grammar it is checked against.
    let (_directory, root) =
        pack_with_tokenization("ignored-node-kinds = [\"container_doc_comment\"]\n");
    let pack = std::sync::Arc::new(ParserPack::load(root).unwrap());
    let error = ParserRuntime::new().unwrap().load(pack).unwrap_err();
    assert!(
        matches!(error, Error::UnknownNodeKind { .. }),
        "expected an unknown node kind, got {error}"
    );
    assert!(
        error.to_string().contains("container_doc_comment"),
        "the error should name the kind: {error}"
    );
}

/// Every pack in the tree, checked against its own grammar.
#[test]
fn every_checked_in_pack_declares_only_kinds_its_grammar_has() {
    let packs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parser-packs");
    let runtime = ParserRuntime::new().unwrap();
    let mut checked = 0;
    let mut directories: Vec<_> = std::fs::read_dir(&packs)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();
    for directory in directories {
        let pack = std::sync::Arc::new(ParserPack::load(&directory).unwrap());
        runtime
            .load(pack)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()));
        checked += 1;
    }
    assert!(checked >= 6, "only {checked} packs were checked");
}
