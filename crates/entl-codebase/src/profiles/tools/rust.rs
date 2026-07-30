use crate::{BINARY_ARTIFACT, RUST_LANGUAGE};

use super::super::{
    CiWorkload, CommandPattern, TaskKind, TestRetryConfiguration, TestRetryProfile,
    TestRetrySignal, ToolProfile, ToolRegistration,
};

const NEXTEST_RETRY_CONFIGURATION: TestRetryConfiguration = TestRetryConfiguration {
    paths: &[".config/nextest.toml"],
    signals: &[TestRetrySignal::TomlPositiveInteger("retries")],
};

static CARGO: ToolProfile = ToolProfile {
    id: "cargo",
    programs: &["cargo"],
    languages: &[&RUST_LANGUAGE],
    commands: &[
        CommandPattern {
            artifacts: &[&BINARY_ARTIFACT],
            ..CommandPattern::tasks(&["test"], &[TaskKind::Test])
        },
        CommandPattern {
            artifacts: &[&BINARY_ARTIFACT],
            ..CommandPattern::tasks(&["nextest"], &[TaskKind::Test])
        },
        CommandPattern::tasks(&["clippy"], &[TaskKind::Lint]),
        CommandPattern::tasks(&["fmt"], &[TaskKind::Format]),
        CommandPattern::produces(&["build"], &[], &[], &[&BINARY_ARTIFACT]),
        CommandPattern::produces(&["install"], &[], &[], &[&BINARY_ARTIFACT]),
        CommandPattern::tasks(&["check"], &[TaskKind::Build]),
    ],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Heavy,
    test_retry: Some(TestRetryProfile {
        arguments: &["--retries"],
        configurations: &[NEXTEST_RETRY_CONFIGURATION],
    }),
};

static RUSTFMT: ToolProfile = ToolProfile {
    id: "rustfmt",
    programs: &["rustfmt"],
    languages: &[&RUST_LANGUAGE],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Format])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

crate::profiles::registry::submit! { ToolRegistration(&CARGO) }
crate::profiles::registry::submit! { ToolRegistration(&RUSTFMT) }
