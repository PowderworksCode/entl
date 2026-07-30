use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use crate::{
    ConventionalCommitEnforcement, ConventionalCommitInventory, ConventionalCommitTarget, Workflow,
};

pub use registry_inventory as registry;

#[derive(Debug, Clone, Copy)]
pub struct CommandMatcher {
    pub program: &'static str,
    pub subcommands: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct PullRequestPatternMatcher {
    pub title_sources: &'static [&'static str],
    pub required_fragments: &'static [&'static str],
    pub validator_fragments: &'static [&'static str],
    pub failure_fragments: &'static [&'static str],
}

impl PullRequestPatternMatcher {
    fn matches(self, run: Option<&str>, env: &BTreeMap<String, String>) -> bool {
        let Some(run) = run else {
            return false;
        };
        let reads_title = self
            .title_sources
            .iter()
            .any(|source| run.contains(source) || env.values().any(|value| value.contains(source)));
        reads_title
            && self
                .required_fragments
                .iter()
                .all(|fragment| run.contains(fragment))
            && self
                .validator_fragments
                .iter()
                .any(|fragment| run.contains(fragment))
            && self
                .failure_fragments
                .iter()
                .any(|fragment| run.contains(fragment))
    }
}

impl CommandMatcher {
    fn matches(self, program: &str, arguments: &[String]) -> bool {
        self.program == program
            && (self.subcommands.is_empty()
                || arguments
                    .first()
                    .is_some_and(|argument| self.subcommands.contains(&argument.as_str())))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConventionalCommitEnforcerProfile {
    pub id: &'static str,
    pub target: ConventionalCommitTarget,
    pub actions: &'static [&'static str],
    pub commands: &'static [CommandMatcher],
    pub pull_request_pattern: Option<PullRequestPatternMatcher>,
}

impl ConventionalCommitEnforcerProfile {
    fn matches_action(self, action: &str) -> bool {
        let action = action.split('@').next().unwrap_or(action);
        self.actions.contains(&action)
    }
}

pub struct ConventionalCommitEnforcerRegistration(pub &'static ConventionalCommitEnforcerProfile);

registry::collect!(ConventionalCommitEnforcerRegistration);

macro_rules! register {
    ($name:ident, $id:literal, $target:ident, $actions:expr, $commands:expr, $pattern:expr) => {
        static $name: ConventionalCommitEnforcerProfile = ConventionalCommitEnforcerProfile {
            id: $id,
            target: ConventionalCommitTarget::$target,
            actions: $actions,
            commands: $commands,
            pull_request_pattern: $pattern,
        };
        registry::submit! { ConventionalCommitEnforcerRegistration(&$name) }
    };
}

register!(
    SEMANTIC_PULL_REQUEST,
    "semantic-pull-request",
    PullRequestTitle,
    &[
        "amannn/action-semantic-pull-request",
        "step-security/action-semantic-pull-request",
    ],
    &[],
    None
);
register!(
    COMMITLINT,
    "commitlint",
    CommitMessage,
    &["wagoid/commitlint-github-action"],
    &[CommandMatcher {
        program: "commitlint",
        subcommands: &[],
    }],
    None
);
register!(
    COCOGITTO,
    "cocogitto",
    CommitMessage,
    &["cocogitto/cocogitto-action"],
    &[
        CommandMatcher {
            program: "cog",
            subcommands: &["check", "verify"],
        },
        CommandMatcher {
            program: "cocogitto",
            subcommands: &["check", "verify"],
        },
    ],
    None
);
register!(
    CONVCO,
    "convco",
    CommitMessage,
    &[],
    &[CommandMatcher {
        program: "convco",
        subcommands: &["check"],
    }],
    None
);
register!(
    CONVENTIONAL_PATTERN,
    "conventional-pr-title-pattern",
    PullRequestTitle,
    &[],
    &[],
    Some(PullRequestPatternMatcher {
        title_sources: &["github.event.pull_request.title"],
        required_fragments: &["feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert",],
        validator_fragments: &["grep", "=~"],
        failure_fragments: &["exit 1", "||"],
    })
);

static REGISTERED: LazyLock<Vec<&'static ConventionalCommitEnforcerProfile>> =
    LazyLock::new(|| {
        let mut profiles = registry::iter::<ConventionalCommitEnforcerRegistration>
            .into_iter()
            .map(|registration| registration.0)
            .collect::<Vec<_>>();
        profiles.sort_by_key(|profile| profile.id);
        let mut ids = BTreeSet::new();
        let mut actions = BTreeSet::new();
        let mut commands = BTreeSet::new();
        for profile in &profiles {
            assert!(ids.insert(profile.id), "duplicate conventional enforcer ID");
            for action in profile.actions {
                assert!(
                    actions.insert(*action),
                    "duplicate conventional enforcer action"
                );
            }
            for command in profile.commands {
                assert!(
                    commands.insert((command.program, command.subcommands)),
                    "duplicate conventional enforcer command"
                );
            }
        }
        profiles
    });

pub fn conventional_commit_enforcer_profiles()
-> &'static [&'static ConventionalCommitEnforcerProfile] {
    REGISTERED.as_slice()
}

pub(crate) fn inspect(workflows: &[Workflow]) -> ConventionalCommitInventory {
    let mut enforcements = Vec::new();
    for workflow in workflows {
        for profile in conventional_commit_enforcer_profiles() {
            if !target_triggered(workflow, profile.target) {
                continue;
            }
            for job in &workflow.jobs {
                for step in &job.steps {
                    let action_match = step
                        .uses
                        .as_deref()
                        .is_some_and(|action| profile.matches_action(action));
                    let custom_match = profile
                        .pull_request_pattern
                        .is_some_and(|pattern| pattern.matches(step.run.as_deref(), &step.env));
                    if action_match || custom_match {
                        enforcements.push(ConventionalCommitEnforcement {
                            workflow: workflow.path.clone(),
                            job: job.id.clone(),
                            step: step.index,
                            enforcer: profile.id.to_owned(),
                            target: profile.target,
                        });
                    }
                }
            }
            for command in &workflow.commands {
                if profile
                    .commands
                    .iter()
                    .any(|matcher| matcher.matches(&command.program, &command.arguments))
                {
                    enforcements.push(ConventionalCommitEnforcement {
                        workflow: workflow.path.clone(),
                        job: command.job.clone(),
                        step: command.step,
                        enforcer: profile.id.to_owned(),
                        target: profile.target,
                    });
                }
            }
        }
    }
    enforcements.sort();
    enforcements.dedup();
    ConventionalCommitInventory { enforcements }
}

fn target_triggered(workflow: &Workflow, target: ConventionalCommitTarget) -> bool {
    match target {
        ConventionalCommitTarget::PullRequestTitle => {
            workflow.triggers.contains("pull_request")
                || workflow.triggers.contains("pull_request_target")
        }
        ConventionalCommitTarget::CommitMessage => workflow.runs_on_changes(),
    }
}
