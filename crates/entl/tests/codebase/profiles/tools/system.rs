// Tests for `src/codebase/profiles/tools/system.rs`.
//
// A tool profile is how a command in a workflow becomes a typed fact. If
// the programs it claims are wrong the tool is simply never recognised,
// and nothing else in the system notices.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::tool_profile;

#[test]
fn system_package_manager_is_registered_with_its_programs() {
    let profile =
        tool_profile("system-package-manager").expect("system-package-manager is registered");
    assert_eq!(profile.id, "system-package-manager");
    assert_eq!(profile.programs, ["apt", "apt-get"]);
}

#[test]
fn docker_is_registered_with_its_programs() {
    let profile = tool_profile("docker").expect("docker is registered");
    assert_eq!(profile.id, "docker");
    assert_eq!(profile.programs, ["docker"]);
}

#[test]
fn shellcheck_is_registered_with_its_programs() {
    let profile = tool_profile("shellcheck").expect("shellcheck is registered");
    assert_eq!(profile.id, "shellcheck");
    assert_eq!(profile.programs, ["shellcheck"]);
}

#[test]
fn zizmor_is_registered_with_its_programs() {
    let profile = tool_profile("zizmor").expect("zizmor is registered");
    assert_eq!(profile.id, "zizmor");
    assert_eq!(profile.programs, ["zizmor"]);
}
