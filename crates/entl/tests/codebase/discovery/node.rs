// Tests for `src/codebase/discovery/node.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn node_workspaces_inherit_the_nearest_manager_and_collect_languages() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "package.json",
        r#"{
  "name": "root",
  "private": true,
  "packageManager": "bun@1.3.0",
  "workspaces": ["packages/*", "!packages/ignored"]
}"#,
    );
    write(temp.path(), "bun.lock", "");
    write(
        temp.path(),
        "packages/app/package.json",
        r#"{"name":"app","devDependencies":{"typescript":"^6"}}"#,
    );
    write(temp.path(), "packages/app/tsconfig.json", "{}\n");
    write(
        temp.path(),
        "packages/app/src/app.tsx",
        "export const App = () => null;\n",
    );
    write(
        temp.path(),
        "packages/ignored/package.json",
        r#"{"name":"ignored"}"#,
    );
    write(
        temp.path(),
        "packages/ignored/index.js",
        "module.exports = {};\n",
    );

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let app = inventory
        .package(&PackageId::from("node:packages/app"))
        .unwrap();
    assert_eq!(app.workspace, Some(WorkspaceId::from("node:.")));
    assert!(app.ecosystems.contains(&EcosystemId::from("bun")));
    assert!(app.has_language("javascript"));
    assert!(app.has_language("typescript"));
    assert!(app.depends_on("typescript"));
    assert_eq!(
        inventory.file("packages/app/src/app.tsx").unwrap().packages,
        [PackageId::from("node:packages/app")]
    );

    let ignored = inventory
        .package(&PackageId::from("node:packages/ignored"))
        .unwrap();
    assert_eq!(ignored.workspace, None);
    assert_eq!(ignored.ecosystem, Some(EcosystemId::from("npm")));
}

#[test]
fn pnpm_workspace_selects_manager_and_project_facts() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "package.json", "{}");
    write(temp.path(), "pnpm-lock.yaml", "");
    write(
        temp.path(),
        "pnpm-workspace.yaml",
        "packages:\n  - packages/*\n  - '!packages/private'\n",
    );
    write(
        temp.path(),
        "packages/site/package.json",
        r#"{"devDependencies":{"typescript":"1","vite":"1"}}"#,
    );
    write(temp.path(), "packages/site/tsconfig.json", "{}");
    write(temp.path(), "packages/private/package.json", "{}");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let site = inventory
        .package(&PackageId::from("node:packages/site"))
        .unwrap();
    assert_eq!(site.ecosystem, Some(EcosystemId::from("pnpm")));
    assert_eq!(site.workspace, Some(WorkspaceId::from("node:.")));
    assert_eq!(site.lockfile_owner, Path::new(""));
    assert_eq!(site.lockfile, Some("pnpm-lock.yaml".into()));
    let project = inventory.project("packages/site").unwrap();
    assert!(project.has_language("typescript"));
    assert!(project.uses_ecosystem("pnpm"));
    assert!(project.has_facet("static-site"));

    let private = inventory
        .package(&PackageId::from("node:packages/private"))
        .unwrap();
    assert_eq!(private.workspace, None);
}
