use crate::codebase::{BINARY_ARTIFACT, RUST_LANGUAGE};

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

// Invoked as `cargo hawk`; normalize_invocation resolves the external cargo
// subcommand to the `cargo-hawk` binary this profile claims. Heavy because it
// drives the compiler over the whole workspace.
pub static HAWK: ToolProfile = ToolProfile {
    id: "hawk",
    programs: &["cargo-hawk"],
    languages: &[&RUST_LANGUAGE],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Heavy,
    test_retry: None,
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

crate::codebase::profiles::registry::submit! { ToolRegistration(&CARGO) }
crate::codebase::profiles::registry::submit! { ToolRegistration(&HAWK) }
crate::codebase::profiles::registry::submit! { ToolRegistration(&RUSTFMT) }
