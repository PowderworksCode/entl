// The mirrored tests for `src/corpus/`, one file per source module.
//
// A test target resolves `mod` against its own directory rather than a
// subdirectory named after it, so each module states its path, and cargo builds
// only top-level files under tests/ as targets.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "corpus/exercism.rs"]
mod exercism;
#[path = "corpus/mal.rs"]
mod mal;
#[path = "corpus/mod.rs"]
mod module;
#[path = "corpus/rosetta.rs"]
mod rosetta;
