// Tests for `src/corpus/mal.rs`.
//
// The reader recognises its corpus by shape. Saying no to a directory that is
// not one is the half worth testing without a checkout: the other half needs
// the corpus itself.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use verbosity::corpus::mal;

#[test]
fn an_empty_directory_is_not_mistaken_for_the_corpus() {
    let temp = tempfile::tempdir().unwrap();
    assert!(mal::read(temp.path()).is_err());
}
