// Tests for `src/codebase/error.rs`.
//
// The messages are the interface: someone reading one is deciding what to fix,
// so each names the thing it is about.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn an_unreadable_root_names_the_path() {
    let missing = std::path::Path::new("/no/such/codebase/root");
    let error =
        inspect(missing, &InventoryOptions::default()).expect_err("a missing root is an error");
    let text = error.to_string();
    assert!(text.contains("/no/such/codebase/root"), "{text}");
}
