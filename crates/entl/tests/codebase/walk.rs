// Tests for `src/codebase/walk.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support::*;

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
