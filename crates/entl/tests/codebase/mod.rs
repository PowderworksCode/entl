// Tests for `src/codebase/mod.rs`: what the codebase module exposes.
//
// The module re-exports the whole observation surface, and every consumer binds
// to those names. Naming them is the test — each line stops compiling if one
// is made private or moved.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn the_observation_surface_is_reachable() {
    let temp = tempfile::tempdir().unwrap();
    inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let _ = InventoryOptions::default();
    let _ = entl::codebase::language_profile("rust");
    let _ = entl::codebase::ecosystem_profile("cargo");
    let _ = entl::codebase::tool_profile("cargo");
    let _ = entl::codebase::language_facets();
    let _ = entl::codebase::artifact_profiles();
}
