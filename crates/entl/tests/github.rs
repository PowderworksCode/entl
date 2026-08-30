// The mirrored tests for `src/github/`, one file per module.
//
// A test target resolves `mod` against its own directory rather than a
// subdirectory named after it, so each module states its path, and cargo
// builds only top-level files under tests/ as targets.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "github/support.rs"]
mod support;

#[path = "github/action.rs"]
mod action;
#[path = "github/automerge.rs"]
mod automerge;
#[path = "github/codeowners.rs"]
mod codeowners;
#[path = "github/conventional.rs"]
mod conventional;
#[path = "github/dependabot.rs"]
mod dependabot;
#[path = "github/workflow.rs"]
mod workflow;
