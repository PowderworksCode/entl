// Tests for `src/codebase/profiles/tools/documentation.rs`.
//
// A tool profile is how a command in a workflow becomes a typed fact. If
// the programs it claims are wrong the tool is simply never recognised,
// and nothing else in the system notices.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::tool_profile;

#[test]
fn codespell_is_registered_with_its_programs() {
    let profile = tool_profile("codespell").expect("codespell is registered");
    assert_eq!(profile.id, "codespell");
    assert_eq!(profile.programs, ["codespell"]);
}

#[test]
fn vale_is_registered_with_its_programs() {
    let profile = tool_profile("vale").expect("vale is registered");
    assert_eq!(profile.id, "vale");
    assert_eq!(profile.programs, ["vale"]);
}
