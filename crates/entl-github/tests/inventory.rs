use std::fs;
use std::path::Path;

use entl_codebase::{
    BUN_ECOSYSTEM, CARGO_ECOSYSTEM, InventoryOptions, TaskKind, inspect as inspect_codebase,
};
use entl_github::{dependabot_ecosystem_profile, inspect};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

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

#[test]
fn invalid_dependabot_configuration_has_scoped_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/dependabot.yaml",
        "version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n",
    );
    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert!(github.dependabot.configuration.is_none());
    assert_eq!(github.dependabot.diagnostics.len(), 1);
    assert!(
        github.dependabot.diagnostics[0]
            .message
            .contains("schedule.interval")
    );
    assert!(github.diagnostics.is_empty());
}

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

#[test]
fn invalid_codeowners_syntax_has_scoped_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "CODEOWNERS",
        "!generated/ @owner\n[ab].rs @owner\n/src/ owner\n/valid/ @org/team\n",
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let configuration = github.codeowners.configuration.unwrap();
    assert_eq!(configuration.rules.len(), 1);
    assert_eq!(github.codeowners.diagnostics.len(), 3);
    assert!(
        github
            .codeowners
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.path == Path::new("CODEOWNERS"))
    );
    assert!(github.diagnostics.is_empty());
}

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

#[test]
fn package_scripts_expand_into_typed_workflow_tasks() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "package.json",
        r#"{
  "scripts": {
    "check": "biome check .",
    "test": "vitest run",
    "types": "tsc --noEmit"
  },
  "devDependencies": { "typescript": "latest" }
}"#,
    );
    write(temp.path(), "tsconfig.json", "{}\n");
    write(temp.path(), "src/index.ts", "export {};\n");
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        r#"on: [push, pull_request]
jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - run: |
          bun run check
          bun run test
          bun run types
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert_eq!(
        codebase.packages[0].script("test").unwrap().command,
        "vitest run"
    );
    assert!(github.has_task("typescript", TaskKind::Test));
    assert!(github.has_task("typescript", TaskKind::Lint));
    assert!(github.has_task("typescript", TaskKind::Format));
    assert!(github.has_task("typescript", TaskKind::Typecheck));
    assert!(
        github
            .task_invocations()
            .filter(|task| task.tool.as_str() == "biome")
            .chain(
                github
                    .task_invocations()
                    .filter(|task| task.tool.as_str() == "javascript-test-runner")
            )
            .chain(
                github
                    .task_invocations()
                    .filter(|task| task.tool.as_str() == "typescript")
            )
            .all(|task| task.evidence.contains(Path::new("package.json")))
    );
}

#[test]
fn transitive_build_scripts_retain_exact_package_targets() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "package.json",
        r#"{
  "scripts": {
    "build:web": "vite build",
    "build:all": "bun run build:web"
  },
  "devDependencies": {"typescript": "latest"}
}"#,
    );
    write(temp.path(), "src/index.ts", "export {};\n");
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        r#"on: pull_request
jobs:
  build:
    steps:
      - run: bun run build:all
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let targets = github
        .task_invocations()
        .filter_map(|task| task.package_script.as_ref())
        .map(|script| (script.package_root.as_path(), script.name.as_str()))
        .collect::<Vec<_>>();
    assert!(targets.contains(&(Path::new(""), "build:all")));
    assert!(targets.contains(&(Path::new(""), "build:web")));
    assert!(
        github
            .task_invocations()
            .any(|task| task.produces_artifact("site"))
    );
}

#[test]
fn package_script_arguments_reach_typed_tools() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "package.json",
        r#"{"scripts":{"tauri":"tauri"},"devDependencies":{"typescript":"latest"}}"#,
    );
    write(temp.path(), "src/index.ts", "export {};\n");
    write(
        temp.path(),
        ".github/workflows/nightly.yml",
        r#"on:
  schedule:
    - cron: "0 0 * * *"
jobs:
  desktop:
    steps:
      - run: bun run tauri build
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let task = github
        .task_invocations()
        .find(|task| task.tool.as_str() == "tauri")
        .unwrap();
    assert_eq!(task.program, "tauri");
    assert_eq!(task.arguments, ["build"]);
    assert_eq!(task.kind, TaskKind::Build);
    assert!(task.produces_artifact("tauri"));
    assert!(task.produces_artifact("site"));
}

#[test]
fn artifact_outputs_are_flag_sensitive_and_transitive() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "package.json",
        r#"{
  "scripts": {
    "build:addon": "napi build --release",
    "build:cli": "bun build src/cli.ts --compile",
    "build:web": "bun build src/web.ts --target browser"
  }
}"#,
    );
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        r#"on: push
jobs:
  build:
    steps:
      - run: |
          bun run build:addon
          bun run build:cli
          bun run build:web
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let outputs = github
        .task_invocations()
        .flat_map(|task| task.artifacts.iter().map(|artifact| artifact.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(outputs, ["binary", "napi", "site"].into());
}

#[test]
fn workflows_retain_unknown_commands_and_step_order() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "package.json", "{}");
    write(
        temp.path(),
        ".github/workflows/codegen.yml",
        r#"on: pull_request
jobs:
  generated:
    steps:
      - run: make bindgen && git diff --exit-code
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let commands = &github.workflows[0].commands;
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].program, "make");
    assert_eq!(commands[0].arguments, ["bindgen"]);
    assert_eq!(commands[0].segment, 0);
    assert_eq!(commands[1].program, "git");
    assert_eq!(commands[1].arguments, ["diff", "--exit-code"]);
    assert_eq!(commands[1].segment, 1);
    assert!(commands[0].package_roots.contains(Path::new("")));
}

#[test]
fn wrappers_and_rust_subcommands_are_classified() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\n",
    );
    write(temp.path(), "src/lib.rs", "pub fn demo() {}\n");
    write(
        temp.path(),
        ".github/workflows/ci.yaml",
        r#"on: push
jobs:
  quality:
    steps:
      - run: cargo nextest run && cargo clippy --all-targets
      - run: cargo fmt --check
      - run: npx tsc --noEmit
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert!(github.has_task("rust", TaskKind::Test));
    assert!(github.has_task("rust", TaskKind::Lint));
    assert!(github.has_task("rust", TaskKind::Format));
    assert!(github.has_task("typescript", TaskKind::Typecheck));
}

#[test]
fn nested_working_directory_selects_the_nearest_package_script() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "package.json", r#"{"private":true}"#);
    write(
        temp.path(),
        "apps/web/package.json",
        r#"{"scripts":{"test":"vitest run"},"devDependencies":{"typescript":"latest"}}"#,
    );
    write(temp.path(), "apps/web/src/index.ts", "export {};\n");
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        r#"on: pull_request
jobs:
  web:
    defaults:
      run:
        working-directory: apps/web
    steps:
      - run: npm test
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let task = github
        .task_invocations()
        .find(|task| task.tool.as_str() == "javascript-test-runner")
        .unwrap();
    assert_eq!(task.working_directory, Path::new("apps/web"));
    assert!(task.evidence.contains(Path::new("apps/web/package.json")));
}

#[test]
fn tasks_outside_change_workflows_do_not_satisfy_inventory_queries() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.0.0\"\n",
    );
    write(temp.path(), "src/lib.rs", "pub fn demo() {}\n");
    write(
        temp.path(),
        ".github/workflows/nightly.yml",
        r#"on:
  schedule:
    - cron: "0 0 * * *"
jobs:
  test:
    steps:
      - run: cargo test
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert!(github.workflows[0].has_task("rust", TaskKind::Test));
    assert!(!github.has_task("rust", TaskKind::Test));
}

#[test]
fn invalid_workflows_are_retained_as_files_and_report_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), ".github/workflows/broken.yml", "jobs: [\n");

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert!(github.has_workflows());
    assert!(github.workflows.is_empty());
    assert_eq!(
        github.diagnostics[0].path,
        Path::new(".github/workflows/broken.yml")
    );
}

#[test]
fn pr_check_jobs_use_display_names_and_exclude_helpers_and_dynamic_names() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        r#"on: {pull_request: {}}
jobs:
  changes:
    outputs: {code: value}
    steps: [{run: echo filter}]
  test:
    name: Tests
    steps: [{run: cargo test}]
  build:
    steps: [{run: cargo build}]
  matrix:
    name: Test ${{ matrix.target }}
    steps: [{run: cargo test}]
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    assert_eq!(
        github.workflows[0].pull_request_checks,
        ["Tests".to_owned(), "build".to_owned()].into()
    );
}

#[test]
fn workflow_jobs_retain_failure_masking_fields() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".github/workflows/ci.yml",
        r#"on: pull_request
jobs:
  test:
    name: Tests
    continue-on-error: true
    timeout-minutes: 15
    steps:
      - name: Run tests
        run: cargo test
        continue-on-error: "true"
      - uses: codecov/codecov-action@v4
        with: {version: 4}
"#,
    );

    let codebase = inspect_codebase(temp.path(), &InventoryOptions::default()).unwrap();
    let github = inspect(&codebase);
    let job = &github.workflows[0].jobs[0];
    assert_eq!(job.id, "test");
    assert_eq!(job.name, "Tests");
    assert!(job.continue_on_error);
    assert_eq!(job.timeout_minutes.as_deref(), Some("15"));
    assert!(job.uses.is_none());
    assert!(job.steps[0].continue_on_error);
    assert_eq!(job.steps[0].label(), "Run tests");
    assert_eq!(job.steps[1].label(), "codecov/codecov-action@v4");
    assert_eq!(job.steps[1].inputs["version"], "4");
}
