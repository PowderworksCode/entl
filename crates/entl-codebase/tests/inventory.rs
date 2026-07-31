use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use entl_codebase::{
    DiagnosticKind, EcosystemId, InventoryOptions, LanguageEvidence, PackageId, SHELL_LANGUAGE,
    WorkspaceId, inspect, walk,
};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn walk_honors_codebase_ignores_but_keeps_hidden_configuration() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), ".gitignore", "ignored/\n");
    write(temp.path(), "Cargo.toml", "[workspace]\n");
    write(temp.path(), "ignored/no.rs", "fn ignored() {}\n");
    write(temp.path(), "target/generated.rs", "fn generated() {}\n");
    write(temp.path(), ".github/workflows/ci.yml", "name: CI\n");
    write(temp.path(), "src/main.rs", "fn main() {}\n");
    write(
        temp.path(),
        "bin/release",
        "#!/usr/bin/env -S python3 -u\nprint('release')\n",
    );
    write(
        temp.path(),
        "bin/setup",
        "#!/usr/bin/env bash\necho setup\n",
    );
    write(temp.path(), "experiments/probe.ts", "export {};\n");

    let inventory = inspect(
        temp.path(),
        &InventoryOptions {
            additional_ignores: vec!["experiments/**".into()],
            ..InventoryOptions::default()
        },
    )
    .unwrap();

    assert!(inventory.has_file(".github/workflows/ci.yml"));
    assert!(inventory.has_file("src/main.rs"));
    assert!(!inventory.has_file("ignored/no.rs"));
    assert!(!inventory.has_file("target/generated.rs"));
    assert!(!inventory.has_file("experiments/probe.ts"));
    let script = inventory.file("bin/release").unwrap();
    assert_eq!(
        script.language.as_ref().unwrap().language.as_str(),
        "python"
    );
    assert!(matches!(
        script.language.as_ref().unwrap().evidence.as_slice(),
        [LanguageEvidence::Shebang { .. }]
    ));
    assert_eq!(
        inventory
            .files_with_language_profile(&SHELL_LANGUAGE)
            .map(|file| file.path.as_path())
            .collect::<Vec<_>>(),
        [Path::new("bin/setup")]
    );
}

#[test]
fn traversal_conventions_require_their_domain_markers() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "notes/build/plan.md", "keep me\n");
    write(temp.path(), "web/package.json", "{}");
    write(temp.path(), "web/build/generated.js", "generated\n");

    let tree = walk(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(tree.has_file("notes/build/plan.md"));
    assert!(!tree.has_file("web/build/generated.js"));
}

#[test]
fn file_walk_is_a_standalone_lazy_layer_with_hidden_file_control() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"not-parsed\"\nversion = \"0.0.0\"\n",
    );
    write(temp.path(), "src/lib.rs", "pub fn visible() {}\n");
    write(temp.path(), ".github/workflows/ci.yml", "name: CI\n");

    let tree = walk(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(tree.has_file("Cargo.toml"));
    assert!(tree.has_file(".github/workflows/ci.yml"));
    assert_eq!(
        tree.read_text("src/lib.rs").unwrap(),
        "pub fn visible() {}\n"
    );
    assert!(tree.read_text("../outside").is_err());

    let without_hidden = walk(
        temp.path(),
        &InventoryOptions {
            include_hidden: false,
            ..InventoryOptions::default()
        },
    )
    .unwrap();
    assert!(!without_hidden.has_file(".github/workflows/ci.yml"));
}

#[test]
fn file_walk_can_inherit_ignore_files_above_its_root() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), ".gitignore", "generated.rs\n");
    write(temp.path(), "nested/kept.rs", "pub fn kept() {}\n");
    write(
        temp.path(),
        "nested/generated.rs",
        "pub fn generated() {}\n",
    );

    let isolated = walk(temp.path().join("nested"), &InventoryOptions::default()).unwrap();
    assert!(isolated.has_file("generated.rs"));

    let inherited = walk(
        temp.path().join("nested"),
        &InventoryOptions {
            respect_parent_ignores: true,
            ..InventoryOptions::default()
        },
    )
    .unwrap();
    assert!(!inherited.has_file("generated.rs"));
    assert!(inherited.has_file("kept.rs"));
}

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
fn content_reads_are_lazy_and_codebase_relative() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "src/lib.rs", "pub const VALUE: u8 = 7;\n");
    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert_eq!(
        inventory.read_text("src/lib.rs").unwrap(),
        "pub const VALUE: u8 = 7;\n"
    );
    assert!(inventory.read_text("../outside").is_err());
    assert!(inventory.read_text(temp.path().join("src/lib.rs")).is_err());
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
