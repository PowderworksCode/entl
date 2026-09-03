// Tests for `src/codebase/profiles/tools/tauri.rs`.
//
// A tool profile is how a command in a workflow becomes a typed fact. If
// the programs it claims are wrong the tool is simply never recognised,
// and nothing else in the system notices.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::tool_profile;

#[test]
fn tauri_is_registered_with_its_programs() {
    let profile = tool_profile("tauri").expect("tauri is registered");
    assert_eq!(profile.id, "tauri");
    assert_eq!(profile.programs, ["tauri"]);
}
