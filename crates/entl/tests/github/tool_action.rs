// Tests for `src/github/tool_action.rs`.
//
// A tool-action profile is how `uses: owner/action@v1` becomes the statement
// that a workflow runs a particular tool. Each profile points at a tool profile
// that has to exist in the codebase registry, which is the join these two
// registries make.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::tool_profile;
use entl::github::tool_action_profiles;

#[test]
fn the_registry_is_not_empty() {
    assert!(!tool_action_profiles().is_empty());
}

/// Every action string is what a workflow would write, and every profile names
/// a tool the codebase side also knows — a dangling one would type a step as a
/// tool nothing else can say anything about.
#[test]
fn every_profile_names_actions_and_a_registered_tool() {
    for profile in tool_action_profiles() {
        assert!(
            !profile.actions.is_empty(),
            "{} claims no actions",
            profile.tool.id
        );
        for action in profile.actions {
            assert!(!action.is_empty());
        }
        assert_eq!(
            tool_profile(profile.tool.id).map(|tool| tool.id),
            Some(profile.tool.id),
            "{} is not in the codebase tool registry",
            profile.tool.id
        );
    }
}
