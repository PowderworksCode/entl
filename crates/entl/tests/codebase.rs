// The mirrored tests for `src/codebase/`, one file per source module.
//
// A test target resolves `mod` against its own directory rather than a
// subdirectory named after it, so each module states its path, and cargo
// builds only top-level files under tests/ as targets.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "codebase/support.rs"]
mod support;

#[path = "codebase/compiler.rs"]
mod compiler;
#[path = "codebase/discovery/cargo.rs"]
mod discovery_cargo;
#[path = "codebase/discovery/mod.rs"]
mod discovery_mod;
#[path = "codebase/discovery/node.rs"]
mod discovery_node;
#[path = "codebase/model/artifact.rs"]
mod model_artifact;
#[path = "codebase/model/file.rs"]
mod model_file;
#[path = "codebase/walk.rs"]
mod walk;
