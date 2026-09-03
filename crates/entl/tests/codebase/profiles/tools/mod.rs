// Tests for `src/codebase/profiles/tools/mod.rs`: the module that declares
// every tool profile.
//
// The declarations and the registry are two lists of the same thing. A profile
// written but never declared registers nothing, and nothing else notices — the
// tool is simply never recognised.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::Path;

use entl::codebase::{tool_profile, tool_profiles};

#[test]
fn every_declared_module_contributes_a_profile() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/codebase/profiles/tools/mod.rs"),
    )
    .unwrap();
    let declared = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("mod ")?.strip_suffix(';'))
        .filter(|module| *module != "syntax")
        .count();
    assert!(declared > 0, "the module declares profiles");
    assert!(
        tool_profiles().len() >= declared,
        "{} declared modules but only {} registered profiles",
        declared,
        tool_profiles().len()
    );
}

/// Sorted, because the lookup binary-searches it.
#[test]
fn the_registry_is_sorted_and_every_entry_is_findable() {
    let profiles = tool_profiles();
    assert!(profiles.windows(2).all(|pair| pair[0].id < pair[1].id));
    for profile in profiles {
        assert_eq!(
            tool_profile(profile.id).map(|found| found.id),
            Some(profile.id)
        );
    }
}
