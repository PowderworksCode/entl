// The mirrored tests for `src/codebase/`, one file per source module.
//
// A test target resolves `mod` against its own directory rather than a
// subdirectory named after it, so each module states its path. Cargo builds only
// top-level files under tests/ as targets, which is why they arrive through
// this one.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "codebase/compiler.rs"]
mod compiler;
