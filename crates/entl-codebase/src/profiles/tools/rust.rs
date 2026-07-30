use crate::{BINARY_ARTIFACT, RUST_LANGUAGE};

use super::super::{CommandPattern, TaskKind, ToolProfile, ToolRegistration};

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
};

static RUSTFMT: ToolProfile = ToolProfile {
    id: "rustfmt",
    programs: &["rustfmt"],
    languages: &[&RUST_LANGUAGE],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Format])],
};

crate::profiles::registry::submit! { ToolRegistration(&CARGO) }
crate::profiles::registry::submit! { ToolRegistration(&RUSTFMT) }
