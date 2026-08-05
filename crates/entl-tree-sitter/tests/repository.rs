#![allow(clippy::unwrap_used, clippy::expect_used)]
//! What `parse_repository` says about the files it did not read.
//!
//! Every way out of the parse loop records a diagnostic except one, and the
//! exception was the case that most needed saying: a file whose language is
//! known but for which no pack is configured. A consumer reporting on
//! diagnostics then described a repository as clean when it had read none of
//! it. Straitjacket printed `ok — no findings in 44 file(s)` over Python
//! before the Python pack existed, and would do the same today for any
//! language the fleet has not onboarded.

use std::path::PathBuf;

use entl_tree_sitter::{ParserCatalog, parse_repository};

fn packs(names: &[&str]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parser-packs");
    let staged = tempfile::Builder::new()
        .prefix("entl-packs")
        .tempdir()
        .unwrap()
        .keep();
    for name in names {
        let destination = staged.join(name);
        std::fs::create_dir_all(&destination).unwrap();
        for entry in std::fs::read_dir(root.join(name)).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                std::fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
            }
        }
    }
    staged
}

fn repository(files: &[(&str, &str)]) -> PathBuf {
    let root = tempfile::Builder::new()
        .prefix("entl-repo")
        .tempdir()
        .unwrap()
        .keep();
    for (name, source) in files {
        std::fs::write(root.join(name), source).unwrap();
    }
    root
}

/// The finding this whole file exists for.
#[test]
fn a_language_with_no_pack_is_reported_rather_than_skipped() {
    let catalog = ParserCatalog::discover([packs(&["rust"])]);
    assert!(catalog.errors.is_empty(), "{:?}", catalog.errors);
    let root = repository(&[
        ("kept.rs", "fn main() {}\n"),
        ("dropped.py", "def f(x):\n    return x\n"),
    ]);

    let parsed = parse_repository(&root, &catalog.catalog).unwrap();

    assert_eq!(parsed.files.len(), 1, "the Rust file is still read");
    let reported: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.path.ends_with("dropped.py"))
        .collect();
    assert_eq!(
        reported.len(),
        1,
        "a Python file with no Python pack must be reported, got {:?}",
        parsed.diagnostics
    );
    assert!(
        reported[0].message.contains("python"),
        "the diagnostic names the language that went unread: {}",
        reported[0].message
    );
}

/// A file nothing claims as source is not a finding.
///
/// The rule is about a language that was RECOGNIZED and then not read. A
/// README has no language, nobody promised to analyze it, and reporting it
/// would bury the case above in noise.
#[test]
fn a_file_that_is_not_source_is_not_reported() {
    let catalog = ParserCatalog::discover([packs(&["rust"])]);
    let root = repository(&[("kept.rs", "fn main() {}\n"), ("README", "hello\n")]);

    let parsed = parse_repository(&root, &catalog.catalog).unwrap();

    assert!(
        parsed.diagnostics.is_empty(),
        "nothing claimed the README, so nothing failed to read it: {:?}",
        parsed.diagnostics
    );
}

/// A recognized language that is not source is not a finding either.
///
/// The previous test covers a file nothing claims. This covers the harder
/// case: TOML, YAML, and Markdown are all registered languages, so they are
/// recognized and then not read, which is literally the shape the rule fires
/// on. They are still not the source under analysis, and only six of the
/// twenty-nine registered languages have a pack — so reporting every one of
/// them meant a repository with a README and a config file produced three
/// findings that named nothing anybody would parse, burying the Go and C++
/// gaps this rule exists to surface.
///
/// Straitjacket found this the expensive way: `exact-clones` reported
/// `analysis-incomplete` against the `straitjacket.toml` that configured the
/// run.
#[test]
fn a_recognized_language_that_is_not_source_is_not_reported() {
    let catalog = ParserCatalog::discover([packs(&["rust"])]);
    let root = repository(&[
        ("kept.rs", "fn main() {}\n"),
        ("README.md", "# hello\n"),
        ("config.toml", "[table]\nkey = 1\n"),
        ("conf.yaml", "key: 1\n"),
    ]);

    let parsed = parse_repository(&root, &catalog.catalog).unwrap();

    assert_eq!(parsed.files.len(), 1, "the Rust file is still read");
    assert!(
        parsed.diagnostics.is_empty(),
        "documentation and data are not source that went unread: {:?}",
        parsed.diagnostics
    );
}

/// With the pack present the file is read, and there is nothing to report.
///
/// This is the half that keeps the rule from being satisfied by reporting
/// every file forever.
#[test]
fn a_language_with_its_pack_reports_nothing() {
    let catalog = ParserCatalog::discover([packs(&["rust", "python"])]);
    assert!(catalog.errors.is_empty(), "{:?}", catalog.errors);
    let root = repository(&[
        ("kept.rs", "fn main() {}\n"),
        ("read.py", "def f(x):\n    return x\n"),
    ]);

    let parsed = parse_repository(&root, &catalog.catalog).unwrap();

    assert_eq!(parsed.files.len(), 2, "both files are read");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}
