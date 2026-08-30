use crate::codebase::SHELL_LANGUAGE;

use super::super::{CiWorkload, CommandPattern, TaskKind, ToolProfile, ToolRegistration};

static SYSTEM_PACKAGES: ToolProfile = ToolProfile {
    id: "system-package-manager",
    programs: &["apt", "apt-get"],
    languages: &[],
    commands: &[CommandPattern::tasks(&["install"], &[TaskKind::Build])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Heavy,
    test_retry: None,
};

static DOCKER: ToolProfile = ToolProfile {
    id: "docker",
    programs: &["docker"],
    languages: &[],
    commands: &[CommandPattern::tasks(&["build"], &[TaskKind::Build])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Heavy,
    test_retry: None,
};

pub static SHELLCHECK: ToolProfile = ToolProfile {
    id: "shellcheck",
    programs: &["shellcheck"],
    languages: &[&SHELL_LANGUAGE],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &[".shellcheckrc"],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

// Static analysis of the workflows themselves; commonly run as `uvx zizmor`,
// which normalize_invocation unwraps like the JavaScript runners.
pub static ZIZMOR: ToolProfile = ToolProfile {
    id: "zizmor",
    programs: &["zizmor"],
    languages: &[],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &["zizmor.yml", ".github/zizmor.yml"],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

crate::codebase::profiles::registry::submit! { ToolRegistration(&SYSTEM_PACKAGES) }
crate::codebase::profiles::registry::submit! { ToolRegistration(&DOCKER) }
crate::codebase::profiles::registry::submit! { ToolRegistration(&SHELLCHECK) }
crate::codebase::profiles::registry::submit! { ToolRegistration(&ZIZMOR) }
