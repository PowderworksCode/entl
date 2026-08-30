// Tests for `src/codebase/profiles/tools/rust.rs`.
//
// A tool profile is how a command in a workflow becomes a typed fact. If
// the programs it claims are wrong the tool is simply never recognised,
// and nothing else in the system notices.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::tool_profile;

#[test]
fn cargo_is_registered_with_its_programs() {
    let profile = tool_profile("cargo").expect("cargo is registered");
    assert_eq!(profile.id, "cargo");
    assert_eq!(profile.programs, ["cargo"]);
}

#[test]
fn hawk_is_registered_with_its_programs() {
    let profile = tool_profile("hawk").expect("hawk is registered");
    assert_eq!(profile.id, "hawk");
    assert_eq!(profile.programs, ["cargo-hawk"]);
}

#[test]
fn rustfmt_is_registered_with_its_programs() {
    let profile = tool_profile("rustfmt").expect("rustfmt is registered");
    assert_eq!(profile.id, "rustfmt");
    assert_eq!(profile.programs, ["rustfmt"]);
}
