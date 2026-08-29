use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;

use entl_codebase::{ArtifactId, LanguageId, TaskKind, ToolId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackageScriptInvocation {
    pub package_root: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskInvocation {
    pub kind: TaskKind,
    pub tool: ToolId,
    pub command: String,
    #[serde(default)]
    pub program: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub package_script: Option<PackageScriptInvocation>,
    #[serde(default)]
    pub package_roots: BTreeSet<PathBuf>,
    pub workflow: PathBuf,
    pub job: String,
    pub step: usize,
    pub working_directory: PathBuf,
    pub languages: BTreeSet<LanguageId>,
    #[serde(default)]
    pub artifacts: BTreeSet<ArtifactId>,
    pub evidence: BTreeSet<PathBuf>,
}

impl TaskInvocation {
    pub fn applies_to_language(&self, language: &str) -> bool {
        self.languages
            .iter()
            .any(|candidate| candidate.as_str() == language)
    }

    pub fn produces_artifact(&self, profile: &str) -> bool {
        self.artifacts
            .iter()
            .any(|artifact| artifact.as_str() == profile)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowToolSource {
    Command,
    GithubAction,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorkflowToolInvocation {
    pub tool: ToolId,
    pub workflow: PathBuf,
    pub job: String,
    pub step: usize,
    pub source: WorkflowToolSource,
    pub runs_on_changes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionPinStatus {
    Pinned,
    Channel,
    Floating,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ActionReference {
    pub workflow: PathBuf,
    pub job: String,
    pub step: usize,
    pub action: String,
    pub reference: Option<String>,
    pub pin_status: ActionPinStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    pub path: PathBuf,
    pub triggers: BTreeSet<String>,
    pub pull_request_path_filters: bool,
    pub pull_request_checks: BTreeSet<String>,
    pub jobs: Vec<WorkflowJob>,
    pub commands: Vec<WorkflowCommand>,
    pub tasks: Vec<TaskInvocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorkflowCommand {
    pub workflow: PathBuf,
    pub job: String,
    pub step: usize,
    pub segment: usize,
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub package_roots: BTreeSet<PathBuf>,
}

impl Workflow {
    pub fn runs_on_changes(&self) -> bool {
        self.triggers.contains("push") || self.triggers.contains("pull_request")
    }

    pub fn has_task(&self, language: &str, kind: TaskKind) -> bool {
        self.tasks
            .iter()
            .any(|task| task.kind == kind && task.applies_to_language(language))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub id: String,
    pub name: String,
    pub condition: Option<String>,
    pub needs: BTreeSet<String>,
    pub has_outputs: bool,
    pub continue_on_error: bool,
    pub timeout_minutes: Option<String>,
    pub uses: Option<String>,
    pub steps: Vec<WorkflowStep>,
    /// Present when the job declares a `strategy.matrix`.
    #[serde(default)]
    pub matrix: Option<WorkflowMatrix>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkflowMatrix {
    /// Job ids whose outputs feed the matrix through a `needs.<id>.outputs`
    /// expression. Empty for a literal matrix.
    pub from_needs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub index: usize,
    pub name: Option<String>,
    pub run: Option<String>,
    pub uses: Option<String>,
    pub inputs: BTreeMap<String, String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub continue_on_error: bool,
}

impl WorkflowStep {
    pub fn label(&self) -> &str {
        self.name
            .as_deref()
            .or(self.run.as_deref())
            .or(self.uses.as_deref())
            .unwrap_or("unnamed step")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DependabotUpdate {
    pub package_ecosystem: String,
    pub directories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependabotConfiguration {
    pub path: PathBuf,
    pub updates: Vec<DependabotUpdate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependabotInventory {
    pub files: BTreeSet<PathBuf>,
    pub configuration: Option<DependabotConfiguration>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CodeownersRule {
    pub line: usize,
    pub pattern: String,
    pub owners: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeownersConfiguration {
    pub path: PathBuf,
    pub rules: Vec<CodeownersRule>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeownersInventory {
    pub files: BTreeSet<PathBuf>,
    pub configuration: Option<CodeownersConfiguration>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConventionalCommitTarget {
    PullRequestTitle,
    CommitMessage,
}

impl ConventionalCommitTarget {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PullRequestTitle => "pull-request titles",
            Self::CommitMessage => "commit messages",
        }
    }
}

impl std::fmt::Display for ConventionalCommitTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConventionalCommitEnforcement {
    pub workflow: PathBuf,
    pub job: String,
    pub step: usize,
    pub enforcer: String,
    pub target: ConventionalCommitTarget,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConventionalCommitInventory {
    pub enforcements: Vec<ConventionalCommitEnforcement>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubInventory {
    #[serde(default)]
    pub codeowners: CodeownersInventory,
    #[serde(default)]
    pub conventional_commits: ConventionalCommitInventory,
    pub dependabot: DependabotInventory,
    pub workflow_files: BTreeSet<PathBuf>,
    pub workflows: Vec<Workflow>,
    #[serde(default)]
    pub tool_invocations: Vec<WorkflowToolInvocation>,
    #[serde(default)]
    pub action_references: Vec<ActionReference>,
    pub diagnostics: Vec<Diagnostic>,
}

impl GithubInventory {
    pub fn has_workflows(&self) -> bool {
        !self.workflow_files.is_empty()
    }

    pub fn task_invocations(&self) -> impl Iterator<Item = &TaskInvocation> {
        self.workflows
            .iter()
            .flat_map(|workflow| workflow.tasks.iter())
    }

    pub fn has_task(&self, language: &str, kind: TaskKind) -> bool {
        has_task(&self.workflows, language, kind)
    }

    pub fn runs_tool(&self, profile: &entl_codebase::ToolProfile) -> bool {
        self.tool_invocations
            .iter()
            .any(|invocation| invocation.runs_on_changes && invocation.tool.as_str() == profile.id)
    }
}

pub fn has_task<'a>(
    workflows: impl IntoIterator<Item = &'a Workflow>,
    language: &str,
    kind: TaskKind,
) -> bool {
    workflows
        .into_iter()
        .filter(|workflow| workflow.runs_on_changes())
        .any(|workflow| workflow.has_task(language, kind))
}

pub(crate) fn deduplicate_tasks(tasks: &mut Vec<TaskInvocation>) {
    let mut seen = HashSet::new();
    tasks.retain(|task| {
        seen.insert((
            task.kind,
            task.tool.clone(),
            task.workflow.clone(),
            task.job.clone(),
            task.step,
            task.working_directory.clone(),
            task.command.clone(),
            task.program.clone(),
            task.arguments.clone(),
            task.package_script.clone(),
            task.package_roots.clone(),
            task.artifacts.clone(),
        ))
    });
    tasks.sort();
}
