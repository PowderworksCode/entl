// Tests for `src/codebase/model/artifact.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

#[test]
fn discovers_distributable_artifacts_with_project_scopes() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "cli/Cargo.toml",
        "[package]\nname='cli'\nversion='0.0.0'\n",
    );
    write(temp.path(), "cli/src/main.rs", "fn main() {}\n");
    write(
        temp.path(),
        "web/package.json",
        r#"{"devDependencies":{"vite":"1","typescript":"1"}}"#,
    );
    write(temp.path(), "web/vite.config.ts", "export default {};\n");
    write(
        temp.path(),
        "native/package.json",
        r#"{"devDependencies":{"@napi-rs/cli":"1"}}"#,
    );
    write(
        temp.path(),
        "desktop/package.json",
        r#"{"devDependencies":{"vite":"1"}}"#,
    );
    write(temp.path(), "desktop/src-tauri/tauri.conf.json", "{}\n");

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let found = inventory
        .artifacts
        .iter()
        .map(|artifact| (artifact.root.as_path(), artifact.profile.as_str()))
        .collect::<BTreeSet<_>>();
    assert!(found.contains(&(Path::new("cli"), "binary")));
    assert!(found.contains(&(Path::new("web"), "site")));
    assert!(found.contains(&(Path::new("native"), "napi")));
    assert!(found.contains(&(Path::new("desktop"), "tauri")));
    assert!(found.contains(&(Path::new("desktop"), "site")));
}

#[test]
fn explicit_cargo_and_bun_binary_targets_are_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "custom/Cargo.toml",
        "[package]\nname='custom'\nversion='0.0.0'\n[[bin]]\nname='custom'\npath='cmd/custom.rs'\n",
    );
    write(temp.path(), "custom/cmd/custom.rs", "fn main() {}\n");
    write(
        temp.path(),
        "bun/package.json",
        r#"{"scripts":{"build:cli":"bun build --compile src/cli.ts"}}"#,
    );

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    let binary_roots = inventory
        .artifacts
        .iter()
        .filter(|artifact| artifact.profile.as_str() == "binary")
        .map(|artifact| artifact.root.as_path())
        .collect::<BTreeSet<_>>();
    assert!(binary_roots.contains(Path::new("custom")));
    assert!(binary_roots.contains(Path::new("bun")));
}
