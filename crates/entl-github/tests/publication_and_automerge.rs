#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::PathBuf;

#[test]
fn action_publication_derives_and_checks_the_marketplace_url() {
    let facts = entl_github::inspect_action_publication(
        PathBuf::from("action.yml"),
        "name: Setup Powderworks\n",
        Some(PathBuf::from("README.md")),
        Some("[Install](https://github.com/marketplace/actions/setup-powderworks)"),
    )
    .unwrap();
    assert_eq!(facts.marketplace_slug, "setup-powderworks");
    assert!(facts.marketplace_linked);
}

#[test]
fn dependabot_automerge_is_reported_as_structured_safeguards() {
    let facts = entl_github::inspect_dependabot_automerge_workflow(
        r#"on: pull_request
jobs:
  automerge:
    if: github.event.pull_request.user.login == 'dependabot[bot]'
    steps:
      - id: meta
        uses: dependabot/fetch-metadata@v2
      - if: steps.meta.outputs.update-type != 'version-update:semver-major'
        run: gh pr merge --auto --squash "$PR_URL"
"#,
    )
    .unwrap();
    assert!(facts.pull_request_trigger);
    assert!(facts.dependabot_only);
    assert!(facts.fetches_metadata);
    assert!(facts.excludes_major_updates);
    assert!(facts.enables_auto_merge);
}

#[test]
fn action_references_are_classified_from_the_explicit_pin_policy() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join(".github/workflows")).unwrap();
    std::fs::write(
        temp.path().join(".github/workflows/ci.yml"),
        "on: push\njobs:\n  ci:\n    steps:\n      - uses: actions/checkout@v4\n      - uses: errata-ai/vale-action@stable\n      - uses: owner/action@0123456789012345678901234567890123456789\n",
    )
    .unwrap();
    let codebase = entl_codebase::inspect(temp.path(), &Default::default()).unwrap();
    let github = entl_github::inspect(&codebase);
    assert_eq!(github.action_references.len(), 3);
    assert_eq!(
        github.action_references[0].pin_status,
        entl_github::ActionPinStatus::Floating
    );
    assert_eq!(
        github.action_references[1].pin_status,
        entl_github::ActionPinStatus::Channel
    );
    assert_eq!(
        github.action_references[2].pin_status,
        entl_github::ActionPinStatus::Pinned
    );
}
