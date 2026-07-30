use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use entl_codebase::{
    ArtifactId, CodebaseInventory, LanguageId, Package, PackageKind, TaskKind, ToolId, Workspace,
    WorkspaceKind, classify_tool, normalize_invocation,
};
use serde_yaml_ng::Value;

use crate::model::deduplicate_tasks;
use crate::{
    Diagnostic, GithubInventory, PackageScriptInvocation, TaskInvocation, Workflow,
    WorkflowCommand, WorkflowJob, WorkflowStep,
};

pub fn inspect(codebase: &CodebaseInventory) -> GithubInventory {
    let codeowners = crate::codeowners::inspect(codebase);
    let dependabot = crate::dependabot::inspect(codebase);
    let workflow_files = codebase
        .files
        .iter()
        .map(|file| &file.path)
        .filter(|path| is_workflow_path(path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut workflows = Vec::new();
    let mut diagnostics = Vec::new();
    for path in &workflow_files {
        match parse_workflow(codebase, path) {
            Ok(workflow) => workflows.push(workflow),
            Err(message) => diagnostics.push(Diagnostic {
                path: path.clone(),
                message,
            }),
        }
    }
    workflows.sort_by(|left, right| left.path.cmp(&right.path));
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.message.cmp(&right.message))
    });
    let conventional_commits = crate::conventional::inspect(&workflows);
    GithubInventory {
        codeowners,
        conventional_commits,
        dependabot,
        workflow_files,
        workflows,
        diagnostics,
    }
}

pub fn pull_request_check_jobs(text: &str) -> Result<BTreeSet<String>, String> {
    let value = serde_yaml_ng::from_str::<Value>(text)
        .map_err(|error| format!("workflow is invalid YAML: {error}"))?;
    Ok(pull_request_check_jobs_from_value(&value))
}

fn is_workflow_path(path: &Path) -> bool {
    path.starts_with(".github/workflows")
        && matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        )
}

fn parse_workflow(codebase: &CodebaseInventory, path: &Path) -> Result<Workflow, String> {
    let text = codebase
        .read_text(path)
        .map_err(|error| format!("workflow is unreadable: {error}"))?;
    let value = serde_yaml_ng::from_str::<Value>(&text)
        .map_err(|error| format!("workflow is invalid YAML: {error}"))?;
    let workflow_triggers = mapping_value(&value, "on")
        .map(triggers)
        .unwrap_or_default();
    let pull_request_checks = pull_request_check_jobs_from_value(&value);
    let mut tasks = Vec::new();
    let mut commands = Vec::new();
    let mut workflow_jobs = Vec::new();
    let jobs = mapping_value(&value, "jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten();
    for (job_id, job) in jobs {
        let Some(job_id) = job_id.as_str() else {
            continue;
        };
        if job.as_mapping().is_none() {
            continue;
        }
        let job_directory = default_working_directory(job).unwrap_or_default();
        let mut workflow_steps = Vec::new();
        let steps = mapping_value(job, "steps")
            .and_then(Value::as_sequence)
            .into_iter()
            .flatten();
        for (step_index, step) in steps.enumerate() {
            if step.as_mapping().is_none() {
                continue;
            }
            workflow_steps.push(WorkflowStep {
                index: step_index,
                name: mapping_value(step, "name")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                run: mapping_value(step, "run")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                uses: mapping_value(step, "uses")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                inputs: mapping_value(step, "with")
                    .and_then(Value::as_mapping)
                    .into_iter()
                    .flatten()
                    .filter_map(|(key, value)| {
                        Some((key.as_str()?.to_owned(), scalar_text(value)?))
                    })
                    .collect(),
                env: mapping_value(step, "env")
                    .and_then(Value::as_mapping)
                    .into_iter()
                    .flatten()
                    .filter_map(|(key, value)| {
                        Some((key.as_str()?.to_owned(), scalar_text(value)?))
                    })
                    .collect(),
                continue_on_error: mapping_value(step, "continue-on-error").is_some_and(is_true),
            });
            let Some(command) = mapping_value(step, "run").and_then(Value::as_str) else {
                continue;
            };
            let working_directory = mapping_value(step, "working-directory")
                .and_then(Value::as_str)
                .and_then(normalize_working_directory)
                .unwrap_or_else(|| job_directory.clone());
            let context = TaskContext {
                workflow: path,
                job: job_id,
                step: step_index,
                working_directory: &working_directory,
                packages: &codebase.packages,
                workspaces: &codebase.workspaces,
            };
            let mut output = ScriptOutput {
                commands: &mut commands,
                tasks: &mut tasks,
            };
            classify_script(
                command,
                &context,
                0,
                &mut HashSet::new(),
                &BTreeSet::new(),
                None,
                &mut output,
            );
        }
        workflow_jobs.push(WorkflowJob {
            id: job_id.to_owned(),
            name: mapping_value(job, "name")
                .and_then(Value::as_str)
                .unwrap_or(job_id)
                .to_owned(),
            continue_on_error: mapping_value(job, "continue-on-error").is_some_and(is_true),
            timeout_minutes: mapping_value(job, "timeout-minutes").and_then(scalar_text),
            uses: mapping_value(job, "uses")
                .and_then(Value::as_str)
                .map(str::to_owned),
            steps: workflow_steps,
        });
    }
    deduplicate_tasks(&mut tasks);
    Ok(Workflow {
        path: path.to_path_buf(),
        triggers: workflow_triggers,
        pull_request_checks,
        jobs: workflow_jobs,
        commands,
        tasks,
    })
}

fn is_true(value: &Value) -> bool {
    value.as_bool() == Some(true)
        || value
            .as_str()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn pull_request_check_jobs_from_value(workflow: &Value) -> BTreeSet<String> {
    let workflow_triggers = mapping_value(workflow, "on")
        .map(triggers)
        .unwrap_or_default();
    if !workflow_triggers.contains("pull_request") {
        return BTreeSet::new();
    }

    mapping_value(workflow, "jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flatten()
        .filter_map(|(job_id, job)| {
            let job_id = job_id.as_str()?;
            if mapping_value(job, "outputs").is_some_and(has_values) {
                return None;
            }
            let name = mapping_value(job, "name")
                .and_then(Value::as_str)
                .unwrap_or(job_id);
            (!name.contains("${{")).then(|| name.to_owned())
        })
        .collect()
}

fn has_values(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::String(value) => !value.is_empty(),
        Value::Sequence(value) => !value.is_empty(),
        Value::Mapping(value) => !value.is_empty(),
        Value::Number(_) | Value::Tagged(_) => true,
    }
}

fn default_working_directory(job: &Value) -> Option<PathBuf> {
    mapping_value(job, "defaults")
        .and_then(|defaults| mapping_value(defaults, "run"))
        .and_then(|run| mapping_value(run, "working-directory"))
        .and_then(Value::as_str)
        .and_then(normalize_working_directory)
}

struct TaskContext<'a> {
    workflow: &'a Path,
    job: &'a str,
    step: usize,
    working_directory: &'a Path,
    packages: &'a [Package],
    workspaces: &'a [Workspace],
}

struct ScriptOutput<'a> {
    commands: &'a mut Vec<WorkflowCommand>,
    tasks: &'a mut Vec<TaskInvocation>,
}

fn classify_script(
    script: &str,
    context: &TaskContext<'_>,
    depth: usize,
    resolving: &mut HashSet<(PathBuf, String)>,
    inherited_evidence: &BTreeSet<PathBuf>,
    inherited_package_root: Option<&Path>,
    output: &mut ScriptOutput<'_>,
) {
    if depth > 8 {
        return;
    }
    for line in script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(tokens) = shell_words::split(line) else {
            continue;
        };
        for command in command_segments(&tokens) {
            let Some((program, arguments)) = normalize_invocation(command) else {
                continue;
            };
            let segment = output
                .commands
                .iter()
                .filter(|command| {
                    command.workflow == context.workflow
                        && command.job == context.job
                        && command.step == context.step
                })
                .count();
            let package_roots = command_package_roots(context, &program, inherited_package_root);
            output.commands.push(WorkflowCommand {
                workflow: context.workflow.to_path_buf(),
                job: context.job.to_owned(),
                step: context.step,
                segment,
                program: program.clone(),
                arguments: arguments.clone(),
                working_directory: context.working_directory.to_path_buf(),
                package_roots: package_roots.clone(),
            });
            for (profile, kind, artifacts) in classify_tool(&program, &arguments) {
                output.tasks.push(task(
                    context,
                    kind,
                    ToolId::from(profile),
                    CommandInvocation {
                        line,
                        program: &program,
                        arguments: &arguments,
                        package_script: None,
                        package_roots: package_roots.clone(),
                    },
                    profile
                        .languages
                        .iter()
                        .map(|language| LanguageId::from(*language)),
                    artifacts.iter().map(|artifact| ArtifactId::from(*artifact)),
                    inherited_evidence.iter().cloned(),
                ));
            }
            let Some((script_name, forwarded_arguments)) =
                package_script_invocation(&program, &arguments)
            else {
                continue;
            };
            let Some(package) = nearest_node_package(context.packages, context.working_directory)
            else {
                continue;
            };
            let Some(package_script) = package.script(script_name) else {
                continue;
            };
            let key = (package.root.clone(), package_script.name.clone());
            if !resolving.insert(key.clone()) {
                continue;
            }
            if let Some(kind) = task_kind_from_script_name(&package_script.name) {
                output.tasks.push(task(
                    context,
                    kind,
                    ToolId::from("package-script"),
                    CommandInvocation {
                        line,
                        program: &program,
                        arguments: &arguments,
                        package_script: Some(PackageScriptInvocation {
                            package_root: package.root.clone(),
                            name: package_script.name.clone(),
                        }),
                        package_roots: BTreeSet::from([package.root.clone()]),
                    },
                    package
                        .languages
                        .iter()
                        .map(|language| language.language.clone()),
                    [],
                    [package.manifest.clone()],
                ));
            }
            let mut script_evidence = inherited_evidence.clone();
            script_evidence.insert(package.manifest.clone());
            let expanded = if forwarded_arguments.is_empty() {
                package_script.command.clone()
            } else {
                format!(
                    "{} {}",
                    package_script.command,
                    forwarded_arguments.join(" ")
                )
            };
            classify_script(
                &expanded,
                context,
                depth + 1,
                resolving,
                &script_evidence,
                Some(&package.root),
                output,
            );
            resolving.remove(&key);
        }
    }
}

struct CommandInvocation<'a> {
    line: &'a str,
    program: &'a str,
    arguments: &'a [String],
    package_script: Option<PackageScriptInvocation>,
    package_roots: BTreeSet<PathBuf>,
}

fn task(
    context: &TaskContext<'_>,
    kind: TaskKind,
    tool: ToolId,
    invocation: CommandInvocation<'_>,
    languages: impl IntoIterator<Item = LanguageId>,
    artifacts: impl IntoIterator<Item = ArtifactId>,
    additional_evidence: impl IntoIterator<Item = PathBuf>,
) -> TaskInvocation {
    let mut evidence = BTreeSet::from([context.workflow.to_path_buf()]);
    evidence.extend(additional_evidence);
    TaskInvocation {
        kind,
        tool,
        command: invocation.line.to_owned(),
        program: invocation.program.to_owned(),
        arguments: invocation.arguments.to_vec(),
        package_script: invocation.package_script,
        package_roots: invocation.package_roots,
        workflow: context.workflow.to_path_buf(),
        job: context.job.to_owned(),
        step: context.step,
        working_directory: context.working_directory.to_path_buf(),
        languages: languages.into_iter().collect(),
        artifacts: artifacts.into_iter().collect(),
        evidence,
    }
}

fn command_segments(tokens: &[String]) -> impl Iterator<Item = &[String]> {
    tokens
        .split(|token| matches!(token.as_str(), "&&" | "||" | ";" | "|"))
        .filter(|segment| !segment.is_empty())
}

fn package_script_invocation<'a>(
    program: &str,
    arguments: &'a [String],
) -> Option<(&'a str, &'a [String])> {
    if !matches!(program, "bun" | "npm" | "pnpm" | "yarn") {
        return None;
    }
    match arguments {
        [run, name, rest @ ..] if run == "run" => Some((name, forwarded_arguments(rest))),
        [name, rest @ ..] => Some((name, forwarded_arguments(rest))),
        _ => None,
    }
}

fn forwarded_arguments(arguments: &[String]) -> &[String] {
    if arguments.first().is_some_and(|argument| argument == "--") {
        &arguments[1..]
    } else {
        arguments
    }
}

fn nearest_node_package<'a>(packages: &'a [Package], directory: &Path) -> Option<&'a Package> {
    packages
        .iter()
        .filter(|package| package.kind == PackageKind::Node && directory.starts_with(&package.root))
        .max_by_key(|package| package.root.components().count())
}

fn nearest_package<'a>(packages: &'a [Package], directory: &Path) -> Option<&'a Package> {
    packages
        .iter()
        .filter(|package| directory.starts_with(&package.root))
        .max_by_key(|package| package.root.components().count())
}

fn command_package_roots(
    context: &TaskContext<'_>,
    program: &str,
    inherited: Option<&Path>,
) -> BTreeSet<PathBuf> {
    if let Some(root) = inherited {
        return BTreeSet::from([root.to_path_buf()]);
    }
    if program == "cargo"
        && let Some(workspace) = context.workspaces.iter().find(|workspace| {
            workspace.kind == WorkspaceKind::Cargo && workspace.root == context.working_directory
        })
    {
        return context
            .packages
            .iter()
            .filter(|package| {
                package.kind == PackageKind::Cargo
                    && (package.root == workspace.root || workspace.members.contains(&package.id))
            })
            .map(|package| package.root.clone())
            .collect();
    }
    nearest_package(context.packages, context.working_directory)
        .map(|package| BTreeSet::from([package.root.clone()]))
        .unwrap_or_default()
}

fn task_kind_from_script_name(name: &str) -> Option<TaskKind> {
    if name == "build" || name.starts_with("build:") || name.ends_with(":build") {
        return Some(TaskKind::Build);
    }
    let base = name.split(':').next().unwrap_or(name);
    match base {
        "test" => Some(TaskKind::Test),
        "lint" => Some(TaskKind::Lint),
        "format" | "fmt" => Some(TaskKind::Format),
        "typecheck" | "type-check" | "check-types" => Some(TaskKind::Typecheck),
        _ => None,
    }
}

fn normalize_working_directory(path: &str) -> Option<PathBuf> {
    if path.contains("${{") {
        return None;
    }
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return None;
    }
    Some(path.to_path_buf())
}

fn triggers(value: &Value) -> BTreeSet<String> {
    match value {
        Value::String(trigger) => BTreeSet::from([trigger.clone()]),
        Value::Sequence(triggers) => triggers
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        Value::Mapping(triggers) => triggers
            .keys()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn mapping_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_mapping()?.get(Value::String(key.to_owned()))
}
