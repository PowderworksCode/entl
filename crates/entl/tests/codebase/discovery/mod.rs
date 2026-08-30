// Tests for `src/codebase/discovery/mod.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn mixed_package_kinds_can_own_the_same_file_without_collapsing() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"native-web\"\nversion = \"0.0.0\"\n",
    );
    write(
        temp.path(),
        "package.json",
        r#"{"name":"native-web","private":true}"#,
    );
    write(temp.path(), "src/lib.rs", "pub fn native() {}\n");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert_eq!(
        inventory.file("src/lib.rs").unwrap().packages,
        [PackageId::from("cargo:."), PackageId::from("node:.")]
    );
}

#[test]
fn malformed_manifests_and_unmatched_members_are_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"missing/*\"]\n",
    );
    write(temp.path(), "app/package.json", "{ definitely not json }");
    write(temp.path(), "README.md", "still inventory me\n");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(inventory.has_file("README.md"));
    assert!(inventory.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::Manifest
            && diagnostic.path == Path::new("app/package.json")
    }));
    assert!(inventory.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == DiagnosticKind::Workspace
            && diagnostic.message.contains("matched no package")
    }));
}

#[test]
fn conflicting_and_unsupported_manager_evidence_is_diagnostic() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "package.json",
        r#"{"packageManager":"mystery@1","workspaces":["packages/["]}"#,
    );
    write(temp.path(), "bun.lock", "");
    write(temp.path(), "pnpm-lock.yaml", "");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(
        inventory
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("unsupported packageManager"))
    );
    assert!(inventory.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("conflicting package-manager lockfiles")
    }));
    assert!(
        inventory
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("invalid workspace pattern"))
    );
}

#[test]
fn source_languages_aggregate_at_the_nearest_project() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "scripts/release.ts", "export {};\n");
    write(
        temp.path(),
        "tools/Cargo.toml",
        "[package]\nname='tool'\nversion='0.0.0'\n",
    );
    write(temp.path(), "tools/src/lib.rs", "pub fn tool() {}\n");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(inventory.project("").unwrap().has_language("typescript"));
    assert!(!inventory.project("").unwrap().has_language("rust"));
    assert!(inventory.project("tools").unwrap().has_language("rust"));
}

#[test]
fn tauri_configuration_marks_the_owning_package() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "apps/desktop/package.json", "{}");
    write(temp.path(), "apps/desktop/src-tauri/tauri.conf.json", "{}");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(
        inventory
            .project("apps/desktop")
            .unwrap()
            .has_facet("tauri")
    );
}
