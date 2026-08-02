#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Per-language data a pack carries that is not expressible as a query.
//!
//! A Tree-sitter query can find a shape. It cannot say that `Result` means a
//! failure can be reported, that `binary_search`'s `Err` is an answer rather
//! than a failure, or that `.unwrap_or_default()` reads identically on a type
//! that never failed. Those are facts about a language's standard library, so
//! they travel with the pack that knows the language.

use std::path::PathBuf;

use entl_tree_sitter::{Error, ParserPack};

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
