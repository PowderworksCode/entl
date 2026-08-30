//! Typed GitHub facts derived from an [`crate::codebase::CodebaseInventory`].
//!
//! This crate recognizes repository-owned GitHub configuration and observed
//! automation tasks. It does not decide which workflows or tasks policy
//! requires.

mod action;
mod automerge;
mod codeowners;
mod conventional;
mod dependabot;
mod model;
mod pin;
mod remote;
mod tool_action;
mod workflow;

pub use action::inspect_action_publication;
pub use automerge::inspect_dependabot_automerge_workflow;
pub use conventional::{
    CommandMatcher as ConventionalCommitCommandMatcher, ConventionalCommitEnforcerProfile,
    ConventionalCommitEnforcerRegistration,
    PullRequestPatternMatcher as ConventionalCommitPullRequestPatternMatcher,
    conventional_commit_enforcer_profiles, registry as conventional_commit_registry,
};
pub use dependabot::{
    DependabotEcosystemProfile, DependabotEcosystemRegistration, dependabot_ecosystem_profile,
    dependabot_ecosystem_profiles, registry as dependabot_registry,
};
pub use model::{
    ActionPinStatus, ActionReference, CodeownersConfiguration, CodeownersInventory, CodeownersRule,
    ConventionalCommitEnforcement, ConventionalCommitInventory, ConventionalCommitTarget,
    DependabotConfiguration, DependabotInventory, DependabotUpdate, Diagnostic, GithubInventory,
    PackageScriptInvocation, TaskInvocation, Workflow, WorkflowCommand, WorkflowJob,
    WorkflowMatrix, WorkflowStep, WorkflowToolInvocation, WorkflowToolSource, has_task,
};
pub use pin::{ACTION_PINS, ActionPinPolicy};
pub use remote::{
    DependabotAutomergeWorkflowFacts, GithubActionPublicationFacts, GithubActionsPermissionsFacts,
    GithubBranchFacts, GithubBranchProtectionFacts, GithubDefaultWorkflowPermissions,
    GithubLicenseFacts, GithubPullRequestAgeFacts, GithubRepositoryFacts, GithubRulesetBypassActor,
    GithubRulesetFacts, GithubSecurityFacts, GithubStaleFacts, GithubValue, GithubWorkflowFacts,
    GithubWorkflowRun,
};
pub use tool_action::{ToolActionProfile, ToolActionRegistration, tool_action_profiles};
pub use workflow::{inspect, pull_request_check_jobs};
