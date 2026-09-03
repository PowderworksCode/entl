// Tests for `src/github/dependabot.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn dependabot_profiles_link_to_codebase_ecosystems() {
    let cargo = dependabot_ecosystem_profile(&CARGO_ECOSYSTEM).unwrap();
    assert_eq!(cargo.package_ecosystem, "cargo");
    let bun = dependabot_ecosystem_profile(&BUN_ECOSYSTEM).unwrap();
    assert!(bun.accepts("bun"));
    assert!(bun.accepts("npm"));
}

#[test]
fn dependabot_configuration_is_typed_separately_from_workflows() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/dependabot.yml",
        r#"version: 2
updates:
  - package-ecosystem: npm
    directories: ["/apps/*", "/packages/**"]
    schedule:
      interval: weekly
"#,
    );
    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let configuration = github.dependabot.configuration.unwrap();
    assert_eq!(configuration.path, Path::new(".github/dependabot.yml"));
    assert_eq!(configuration.updates[0].package_ecosystem, "npm");
    assert_eq!(
        configuration.updates[0].directories,
        ["/apps/*", "/packages/**"]
    );
    assert!(github.diagnostics.is_empty());
    assert!(github.dependabot.diagnostics.is_empty());
}
