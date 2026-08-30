// Tests for `src/codebase/model/workspace.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::WorkspaceKind;

/// The same reasoning as package kinds: these spellings are read by other
/// tools, so they are a format rather than a detail.
#[test]
fn every_workspace_kind_has_a_stable_spelling() {
    for kind in [WorkspaceKind::Cargo, WorkspaceKind::Node] {
        assert!(!kind.as_str().is_empty());
    }
    assert_ne!(WorkspaceKind::Cargo.as_str(), WorkspaceKind::Node.as_str());
}
