// Tests for `src/codebase/discovery/cargo.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn cargo_packages_and_workspace_members_are_separate_facts() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        r#"
[package]
name = "root"
version = "0.0.0"

[workspace]
members = ["crates/*"]
exclude = ["crates/skipped"]

[dependencies]
serde = "1"
"#,
    );
    write(temp.path(), "src/lib.rs", "pub fn root() {}\n");
    write(
        temp.path(),
        "crates/core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.0.0\"\n",
    );
    write(temp.path(), "crates/core/src/lib.rs", "pub fn core() {}\n");
    write(
        temp.path(),
        "crates/skipped/Cargo.toml",
        "[package]\nname = \"skipped\"\nversion = \"0.0.0\"\n",
    );
    write(
        temp.path(),
        "crates/skipped/src/lib.rs",
        "pub fn skipped() {}\n",
    );

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert_eq!(inventory.packages.len(), 3);
    let root = inventory.package(&PackageId::from("cargo:.")).unwrap();
    let core = inventory
        .package(&PackageId::from("cargo:crates/core"))
        .unwrap();
    let skipped = inventory
        .package(&PackageId::from("cargo:crates/skipped"))
        .unwrap();
    assert!(root.depends_on("serde"));
    assert!(root.has_language("rust"));
    assert_eq!(root.workspace, Some(WorkspaceId::from("cargo:.")));
    assert_eq!(core.workspace, Some(WorkspaceId::from("cargo:.")));
    assert_eq!(skipped.workspace, None);

    let workspace = inventory.workspace(&WorkspaceId::from("cargo:.")).unwrap();
    assert_eq!(
        workspace.members,
        [
            PackageId::from("cargo:."),
            PackageId::from("cargo:crates/core")
        ]
    );
    assert_eq!(
        inventory.file("crates/core/src/lib.rs").unwrap().packages,
        [PackageId::from("cargo:crates/core")]
    );
}

#[test]
fn explicit_cargo_workspace_links_control_lockfile_ownership() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "Cargo.toml", "[workspace]\n");
    write(temp.path(), "Cargo.lock", "");
    write(
        temp.path(),
        "tools/member/Cargo.toml",
        "[package]\nname='member'\nversion='0.0.0'\nworkspace='../..'\n",
    );

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let member = inventory
        .package(&PackageId::from("cargo:tools/member"))
        .unwrap();
    assert_eq!(member.workspace, Some(WorkspaceId::from("cargo:.")));
    assert_eq!(member.lockfile_owner, Path::new(""));
    assert_eq!(member.lockfile, Some("Cargo.lock".into()));
    assert!(inventory.project("").unwrap().has_facet("cargo-workspace"));
}

#[test]
fn cargo_lockfiles_expose_exact_resolved_packages() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname='app'\nversion='0.0.0'\n[dependencies]\niter={ package='itertools', version='0.14' }\n",
    );
    write(
        temp.path(),
        "Cargo.lock",
        r#"version = 4

[[package]]
name = "app"
version = "0.0.0"
dependencies = ["itertools"]

[[package]]
name = "itertools"
version = "0.14.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1234567890abcdef"
"#,
    );

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let dependency = &inventory.packages[0].dependencies[0];
    assert_eq!(dependency.name, "iter");
    assert_eq!(dependency.package.as_deref(), Some("itertools"));
    assert_eq!(dependency.package_name(), "itertools");
    let resolution = inventory.dependency_resolutions.first().unwrap();
    assert_eq!(resolution.ecosystem, EcosystemId::from("cargo"));
    assert_eq!(resolution.lockfile, Path::new("Cargo.lock"));
    let itertools = resolution
        .packages
        .iter()
        .find(|package| package.name == "itertools")
        .unwrap();
    assert_eq!(itertools.version, "0.14.0");
    assert_eq!(itertools.checksum.as_deref(), Some("1234567890abcdef"));
}
