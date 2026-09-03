// Tests for `src/corpus/mod.rs`: which corpus is being read.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use verbosity::corpus::{Source, read, revision};

/// Reading a directory that is not the corpus it was told to expect is an
/// error rather than an empty result: an empty corpus and a wrong path would
/// otherwise produce the same table.
#[test]
fn reading_a_directory_that_is_not_the_corpus_is_an_error() {
    let temp = tempfile::tempdir().unwrap();
    for source in [Source::Rosetta, Source::Exercism, Source::Mal] {
        assert!(
            read(source, temp.path()).is_err(),
            "{source:?} should not read an empty directory as a corpus"
        );
    }
}

#[test]
fn a_revision_is_reported_for_a_directory_that_is_not_a_checkout() {
    let temp = tempfile::tempdir().unwrap();
    let _ = revision(temp.path());
}
