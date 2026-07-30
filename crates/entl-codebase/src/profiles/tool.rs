use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::{ArtifactProfile, LanguageProfile, registry};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolId(String);

impl ToolId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ToolId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ToolId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for ToolId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskKind {
    Test,
    Lint,
    Format,
    Typecheck,
    Build,
}

impl TaskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Lint => "lint",
            Self::Format => "format",
            Self::Typecheck => "typecheck",
            Self::Build => "build",
        }
    }
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CommandPattern {
    pub arguments: &'static [&'static str],
    pub required_arguments: &'static [ArgumentPattern],
    pub rejected_arguments: &'static [ArgumentPattern],
    pub tasks: &'static [TaskKind],
    pub artifacts: &'static [&'static ArtifactProfile],
}

impl CommandPattern {
    pub const fn tasks(arguments: &'static [&'static str], tasks: &'static [TaskKind]) -> Self {
        Self {
            arguments,
            required_arguments: &[],
            rejected_arguments: &[],
            tasks,
            artifacts: &[],
        }
    }

    pub const fn produces(
        arguments: &'static [&'static str],
        required_arguments: &'static [ArgumentPattern],
        rejected_arguments: &'static [ArgumentPattern],
        artifacts: &'static [&'static ArtifactProfile],
    ) -> Self {
        Self {
            arguments,
            required_arguments,
            rejected_arguments,
            tasks: &[TaskKind::Build],
            artifacts,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ArgumentPattern {
    Exact(&'static str),
    Prefix(&'static str),
}

impl ArgumentPattern {
    fn matches(self, argument: &str) -> bool {
        match self {
            Self::Exact(expected) => argument == expected,
            Self::Prefix(expected) => argument.starts_with(expected),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToolProfile {
    pub id: &'static str,
    pub programs: &'static [&'static str],
    pub languages: &'static [&'static LanguageProfile],
    pub commands: &'static [CommandPattern],
}

impl From<&ToolProfile> for ToolId {
    fn from(profile: &ToolProfile) -> Self {
        Self::new(profile.id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToolRegistration(pub &'static ToolProfile);

registry::collect!(ToolRegistration);

static REGISTERED: LazyLock<Vec<&'static ToolProfile>> = LazyLock::new(|| {
    let mut profiles = registry::iter::<ToolRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.id);
    for pair in profiles.windows(2) {
        assert_ne!(pair[0].id, pair[1].id, "duplicate tool profile ID");
    }
    let mut programs = BTreeSet::new();
    for profile in &profiles {
        for program in profile.programs {
            assert!(
                programs.insert(*program),
                "duplicate tool program {program:?}"
            );
        }
    }
    profiles
});

pub fn tool_profiles() -> &'static [&'static ToolProfile] {
    REGISTERED.as_slice()
}

pub fn tool_profile(id: &str) -> Option<&'static ToolProfile> {
    tool_profiles()
        .binary_search_by_key(&id, |profile| profile.id)
        .ok()
        .map(|index| tool_profiles()[index])
}

pub fn classify_tool(
    program: &str,
    arguments: &[String],
) -> Vec<(
    &'static ToolProfile,
    TaskKind,
    &'static [&'static ArtifactProfile],
)> {
    let Some(profile) = tool_profiles()
        .iter()
        .copied()
        .find(|profile| profile.programs.contains(&program))
    else {
        return Vec::new();
    };
    profile
        .commands
        .iter()
        .filter(|command| {
            arguments.len() >= command.arguments.len()
                && arguments
                    .iter()
                    .map(String::as_str)
                    .zip(command.arguments.iter().copied())
                    .all(|(actual, expected)| actual == expected)
                && command
                    .required_arguments
                    .iter()
                    .all(|required| arguments.iter().any(|argument| required.matches(argument)))
                && command
                    .rejected_arguments
                    .iter()
                    .all(|rejected| arguments.iter().all(|argument| !rejected.matches(argument)))
        })
        .flat_map(|command| {
            command
                .tasks
                .iter()
                .copied()
                .map(|task| (profile, task, command.artifacts))
        })
        .collect()
}

pub fn normalize_invocation(tokens: &[String]) -> Option<(String, Vec<String>)> {
    let mut tokens = tokens;
    while tokens
        .first()
        .is_some_and(|token| token.contains('=') && !token.starts_with('='))
    {
        tokens = &tokens[1..];
    }
    if tokens.first().is_some_and(|token| token == "env") {
        tokens = &tokens[1..];
        while tokens
            .first()
            .is_some_and(|token| token.contains('=') && !token.starts_with('='))
        {
            tokens = &tokens[1..];
        }
    }
    let program = executable_name(tokens.first()?)?;
    let arguments = tokens[1..].to_vec();
    match (program.as_str(), arguments.as_slice()) {
        ("npx" | "bunx", [wrapped, rest @ ..]) => Some((executable_name(wrapped)?, rest.to_vec())),
        ("npm" | "pnpm", [exec, wrapped, rest @ ..]) if exec == "exec" => {
            Some((executable_name(wrapped)?, rest.to_vec()))
        }
        ("yarn", [dlx, wrapped, rest @ ..]) if dlx == "dlx" => {
            Some((executable_name(wrapped)?, rest.to_vec()))
        }
        _ => Some((program, arguments)),
    }
}

fn executable_name(program: &str) -> Option<String> {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}
