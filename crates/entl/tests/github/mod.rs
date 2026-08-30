// Tests for `src/github/mod.rs`: what the GitHub module exposes.
//
// The module re-exports the inventory surface and the registries over it, and
// every consumer binds to those names.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn the_inventory_surface_is_reachable() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "README.md", "# Fixture\n");
    let codebase =
        entl::codebase::inspect(temp.path(), &entl::codebase::InventoryOptions::default()).unwrap();
    let inventory = entl::github::inspect(&codebase);
    assert!(inventory.workflows.is_empty());
    assert!(!entl::github::tool_action_profiles().is_empty());
}
