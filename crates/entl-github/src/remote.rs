use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "availability", rename_all = "kebab-case")]
pub enum GithubValue<T> {
    Known { value: T },
    Unavailable { reason: String },
}

impl<T> GithubValue<T> {
    pub fn known(value: T) -> Self {
        Self::Known { value }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubRepositoryFacts {
    pub repository: String,
    pub default_branch: String,
    pub visibility: String,
    pub archived: bool,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<GithubLicenseFacts>,
    pub topics: Vec<String>,
    pub has_issues: bool,
    pub allow_auto_merge: bool,
    pub delete_branch_on_merge: bool,
    pub allow_update_branch: bool,
    pub branch: GithubBranchFacts,
    pub security: GithubValue<GithubSecurityFacts>,
    pub vulnerability_alerts: GithubValue<bool>,
    pub automated_security_fixes: GithubValue<bool>,
    pub actions_permissions: GithubValue<GithubActionsPermissionsFacts>,
    pub rulesets: GithubValue<Vec<GithubRulesetFacts>>,
    pub pull_request_checks: GithubValue<Vec<String>>,
    pub workflows: Vec<GithubWorkflowFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubRulesetFacts {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub enforcement: String,
    pub rule_types: BTreeSet<String>,
    pub bypass_actors: Vec<GithubRulesetBypassActor>,
}

impl GithubRulesetFacts {
    pub fn is_active_gating_branch_ruleset(&self) -> bool {
        self.target == "branch"
            && self.enforcement == "active"
            && self
                .rule_types
                .iter()
                .any(|rule| matches!(rule.as_str(), "pull_request" | "required_status_checks"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubRulesetBypassActor {
    pub actor_id: Option<u64>,
    pub actor_type: String,
    pub bypass_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubLicenseFacts {
    pub key: String,
    pub name: String,
    pub spdx_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubBranchFacts {
    pub protected: bool,
    pub protection: GithubValue<GithubBranchProtectionFacts>,
    pub required_checks: GithubValue<Vec<String>>,
    pub strict_status_checks: GithubValue<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubBranchProtectionFacts {
    pub pull_requests_required: bool,
    pub force_pushes_blocked: bool,
    pub deletion_blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubSecurityFacts {
    pub secret_scanning: bool,
    pub push_protection: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GithubDefaultWorkflowPermissions {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubActionsPermissionsFacts {
    pub default_workflow_permissions: GithubDefaultWorkflowPermissions,
    pub can_approve_pull_request_reviews: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubWorkflowFacts {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    pub latest_run: Option<GithubWorkflowRun>,
}

impl GithubWorkflowFacts {
    pub fn is_github_managed(&self) -> bool {
        self.path.starts_with("dynamic/")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubWorkflowRun {
    pub id: u64,
    pub conclusion: Option<String>,
    pub html_url: String,
}
