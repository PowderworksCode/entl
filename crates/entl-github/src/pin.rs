use std::path::PathBuf;

use crate::{ActionPinStatus, ActionReference, Workflow};

pub struct ActionPinPolicy {
    pub commit_sha_length: usize,
    pub allowed_channels: &'static [&'static str],
}

pub static ACTION_PINS: ActionPinPolicy = ActionPinPolicy {
    commit_sha_length: 40,
    allowed_channels: &["stable", "oldstable"],
};

pub(crate) fn references(workflows: &[Workflow]) -> Vec<ActionReference> {
    let mut references = Vec::new();
    for workflow in workflows {
        for job in &workflow.jobs {
            for step in &job.steps {
                let Some(uses) = step.uses.as_deref() else {
                    continue;
                };
                references.push(reference(
                    workflow.path.clone(),
                    job.id.clone(),
                    step.index,
                    uses,
                ));
            }
        }
    }
    references.sort();
    references
}

fn reference(workflow: PathBuf, job: String, step: usize, uses: &str) -> ActionReference {
    if uses.starts_with("./") || uses.starts_with("docker://") {
        return ActionReference {
            workflow,
            job,
            step,
            action: uses.to_owned(),
            reference: None,
            pin_status: ActionPinStatus::Local,
        };
    }
    let (action, reference) = uses.rsplit_once('@').unwrap_or((uses, ""));
    let pin_status = if reference.len() == ACTION_PINS.commit_sha_length
        && reference.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        ActionPinStatus::Pinned
    } else if ACTION_PINS
        .allowed_channels
        .iter()
        .any(|channel| reference.eq_ignore_ascii_case(channel))
    {
        ActionPinStatus::Channel
    } else {
        ActionPinStatus::Floating
    };
    ActionReference {
        workflow,
        job,
        step,
        action: action.to_owned(),
        reference: (!reference.is_empty()).then(|| reference.to_owned()),
        pin_status,
    }
}
