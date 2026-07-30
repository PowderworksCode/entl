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

crate::profiles::registry::submit! { ToolRegistration(&SYSTEM_PACKAGES) }
crate::profiles::registry::submit! { ToolRegistration(&DOCKER) }
