use super::super::{CiWorkload, CommandPattern, TaskKind, ToolProfile, ToolRegistration};

pub static CODESPELL: ToolProfile = ToolProfile {
    id: "codespell",
    programs: &["codespell"],
    languages: &[],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

pub static VALE: ToolProfile = ToolProfile {
    id: "vale",
    programs: &["vale"],
    languages: &[],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &[".vale.ini"],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

crate::codebase::profiles::registry::submit! { ToolRegistration(&CODESPELL) }
crate::codebase::profiles::registry::submit! { ToolRegistration(&VALE) }
