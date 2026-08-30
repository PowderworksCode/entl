// Tests for `src/github/codeowners.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn codeowners_uses_github_precedence_and_retains_typed_rules() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "CODEOWNERS", "* @root-owner\n");
    write(temp.path(), "docs/CODEOWNERS", "* @docs-owner\n");
    write(
        temp.path(),
        ".github/CODEOWNERS",
        "# ownership\n/src/ @org/rust-team maintainer@example.com # rationale\n/apps/github\n",
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let configuration = github.codeowners.configuration.unwrap();
    assert_eq!(configuration.path, Path::new(".github/CODEOWNERS"));
    assert_eq!(configuration.rules.len(), 2);
    assert_eq!(configuration.rules[0].line, 2);
    assert_eq!(configuration.rules[0].pattern, "/src/");
    assert_eq!(
        configuration.rules[0].owners,
        ["@org/rust-team", "maintainer@example.com"]
    );
    assert_eq!(configuration.rules[1].pattern, "/apps/github");
    assert!(configuration.rules[1].owners.is_empty());
    assert_eq!(github.codeowners.files.len(), 3);
    assert!(github.codeowners.diagnostics.is_empty());
}
