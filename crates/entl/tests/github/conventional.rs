// Tests for `src/github/conventional.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn conventional_enforcers_are_typed_with_workflow_provenance() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/workflows/titles.yml",
        r#"on: pull_request_target
jobs:
  title:
    steps:
      - uses: amannn/action-semantic-pull-request@v6
"#,
    );
    write(
        temp.path(),
        ".github/workflows/commits.yml",
        r#"on: pull_request
jobs:
  lint:
    steps:
      - run: npx commitlint --from origin/main --to HEAD
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert_eq!(github.conventional_commits.enforcements.len(), 2);
    assert!(
        github
            .conventional_commits
            .enforcements
            .iter()
            .any(|enforcement| {
                enforcement.enforcer == "semantic-pull-request"
                    && enforcement.workflow == Path::new(".github/workflows/titles.yml")
                    && enforcement.job == "title"
                    && enforcement.step == 0
            })
    );
    assert!(
        github
            .conventional_commits
            .enforcements
            .iter()
            .any(|enforcement| {
                enforcement.enforcer == "commitlint"
                    && enforcement.workflow == Path::new(".github/workflows/commits.yml")
            })
    );
}

#[test]
fn explicit_pr_title_patterns_are_enforcement_but_labels_are_not() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/workflows/conventional.yml",
        r#"name: conventional
on: pull_request
jobs:
  title:
    steps:
      - name: PR title follows conventional commits
        env:
          TITLE: ${{ github.event.pull_request.title }}
        run: |
          echo "$TITLE" | grep -qE '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert): .+' || exit 1
  label:
    steps:
      - name: conventional label
        run: echo conventional
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert_eq!(github.workflows[0].jobs[0].steps[0].env.len(), 1);
    assert_eq!(github.conventional_commits.enforcements.len(), 1);
    assert_eq!(
        github.conventional_commits.enforcements[0].enforcer,
        "conventional-pr-title-pattern"
    );

    write(
        temp.path(),
        ".github/workflows/conventional.yml",
        "name: conventional\non: pull_request\njobs: {}\n",
    );
    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert!(github.conventional_commits.enforcements.is_empty());
}
