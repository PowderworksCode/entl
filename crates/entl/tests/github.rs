// The mirrored tests for src/github/, one file per source module.
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
#[path = "github/model.rs"]
mod model;
#[path = "github/mod.rs"]
mod module;
#[path = "github/pin.rs"]
mod pin;
#[path = "github/remote.rs"]
mod remote;
#[path = "github/tool_action.rs"]
mod tool_action;
#[path = "github/workflow.rs"]
mod workflow;
