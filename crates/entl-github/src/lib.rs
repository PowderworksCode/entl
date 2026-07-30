//! Typed GitHub facts derived from an [`entl_codebase::CodebaseInventory`].
//!
//! This crate recognizes repository-owned GitHub configuration and observed
//! automation tasks. It does not decide which workflows or tasks policy
//! requires.

mod codeowners;
mod conventional;
mod dependabot;
mod model;
mod remote;
mod workflow;

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
    CodeownersConfiguration, CodeownersInventory, CodeownersRule, ConventionalCommitEnforcement,
    ConventionalCommitInventory, ConventionalCommitTarget, DependabotConfiguration,
    DependabotInventory, DependabotUpdate, Diagnostic, GithubInventory, PackageScriptInvocation,
    TaskInvocation, Workflow, WorkflowCommand, WorkflowJob, WorkflowStep, has_task,
};
pub use remote::{
    GithubActionsPermissionsFacts, GithubBranchFacts, GithubBranchProtectionFacts,
    GithubDefaultWorkflowPermissions, GithubRepositoryFacts, GithubSecurityFacts, GithubValue,
    GithubWorkflowFacts, GithubWorkflowRun,
};
pub use workflow::{inspect, pull_request_check_jobs};
