// Tests for `src/github/automerge.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn dependabot_automerge_is_reported_as_structured_safeguards() {
    let facts = entl::github::inspect_dependabot_automerge_workflow(
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
