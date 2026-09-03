// Tests for `src/github/model.rs`: the typed shapes a workflow becomes.
//
// These are what a step is turned into once it has been recognised, and every
// consumer reads them rather than the YAML.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn a_workflow_step_becomes_a_typed_invocation() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        "on: pull_request\njobs:\n  build:\n    steps:\n      - run: cargo test\n",
    );
    let codebase =
        entl::codebase::inspect(temp.path(), &entl::codebase::InventoryOptions::default()).unwrap();
    let inventory = entl::github::inspect(&codebase);
    let workflow = inventory.workflows.first().expect("the workflow is read");
    let job = workflow.jobs.first().expect("the job is read");
    assert!(
        !job.steps.is_empty(),
        "the step survives into the typed model"
    );
}
