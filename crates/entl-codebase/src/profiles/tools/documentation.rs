use super::super::{CommandPattern, TaskKind, ToolProfile, ToolRegistration};

pub static CODESPELL: ToolProfile = ToolProfile {
    id: "codespell",
    programs: &["codespell"],
    languages: &[],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
};

pub static VALE: ToolProfile = ToolProfile {
    id: "vale",
    programs: &["vale"],
    languages: &[],
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
};

crate::profiles::registry::submit! { ToolRegistration(&CODESPELL) }
crate::profiles::registry::submit! { ToolRegistration(&VALE) }
