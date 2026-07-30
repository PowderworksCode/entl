use std::collections::BTreeSet;
use std::sync::LazyLock;

use entl_codebase::{CODESPELL, ToolId, ToolProfile, VALE};

use crate::{Workflow, WorkflowToolInvocation, WorkflowToolSource};

pub struct ToolActionProfile {
    pub tool: &'static ToolProfile,
    pub actions: &'static [&'static str],
}

pub struct ToolActionRegistration(pub &'static ToolActionProfile);

registry_inventory::collect!(ToolActionRegistration);

static CODESPELL_ACTION: ToolActionProfile = ToolActionProfile {
    tool: &CODESPELL,
    actions: &["codespell-project/actions-codespell"],
};

static VALE_ACTION: ToolActionProfile = ToolActionProfile {
    tool: &VALE,
    actions: &["errata-ai/vale-action"],
};

registry_inventory::submit! { ToolActionRegistration(&CODESPELL_ACTION) }
registry_inventory::submit! { ToolActionRegistration(&VALE_ACTION) }

static PROFILES: LazyLock<Vec<&'static ToolActionProfile>> = LazyLock::new(|| {
    let mut profiles = registry_inventory::iter::<ToolActionRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.tool.id);
    let mut actions = BTreeSet::new();
    for profile in &profiles {
        for action in profile.actions {
            assert!(
                actions.insert(action.to_ascii_lowercase()),
                "duplicate GitHub Action tool registration: {action}"
            );
        }
    }
    profiles
});

pub fn tool_action_profiles() -> &'static [&'static ToolActionProfile] {
    &PROFILES
}

pub(crate) fn invocations(workflows: &[Workflow]) -> Vec<WorkflowToolInvocation> {
    let mut invocations = BTreeSet::new();
    for workflow in workflows {
        let runs_on_changes = workflow.runs_on_changes();
        for task in &workflow.tasks {
            invocations.insert(WorkflowToolInvocation {
                tool: task.tool.clone(),
                workflow: workflow.path.clone(),
                job: task.job.clone(),
                step: task.step,
                source: WorkflowToolSource::Command,
                runs_on_changes,
            });
        }
        for job in &workflow.jobs {
            for step in &job.steps {
                let Some(action) = step.uses.as_deref().and_then(|uses| uses.split('@').next())
                else {
                    continue;
                };
                for profile in tool_action_profiles() {
                    if profile
                        .actions
                        .iter()
                        .any(|expected| expected.eq_ignore_ascii_case(action))
                    {
                        invocations.insert(WorkflowToolInvocation {
                            tool: ToolId::from(profile.tool),
                            workflow: workflow.path.clone(),
                            job: job.id.clone(),
                            step: step.index,
                            source: WorkflowToolSource::GithubAction,
                            runs_on_changes,
                        });
                    }
                }
            }
        }
    }
    invocations.into_iter().collect()
}
