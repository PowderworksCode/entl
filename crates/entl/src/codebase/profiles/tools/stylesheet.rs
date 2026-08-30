use crate::codebase::{CSS_LANGUAGE, LESS_LANGUAGE, SCSS_LANGUAGE};

use super::super::{CiWorkload, CommandPattern, TaskKind, ToolProfile, ToolRegistration};

pub static STYLELINT: ToolProfile = ToolProfile {
    id: "stylelint",
    programs: &["stylelint"],
    languages: &[&CSS_LANGUAGE, &SCSS_LANGUAGE, &LESS_LANGUAGE],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &[
        ".stylelintrc",
        ".stylelintrc.json",
        ".stylelintrc.yaml",
        ".stylelintrc.yml",
        ".stylelintrc.js",
        ".stylelintrc.cjs",
        ".stylelintrc.mjs",
        "stylelint.config.js",
        "stylelint.config.cjs",
        "stylelint.config.mjs",
    ],
    package_json_keys: &["stylelint"],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

crate::codebase::profiles::registry::submit! { ToolRegistration(&STYLELINT) }
