use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use super::{
    DiscoveryBuilder, DiscoveryHandler, DiscoveryHandlerRegistration, DiscoveryPhase,
    ManifestFacts, WorkspaceSpec, package_id, registry, workspace_id,
};
use crate::{
    Dependency, DependencyKind, DependencySource, Diagnostic, DiagnosticKind, Manifest,
    ManifestKind, Package, PackageKind, PackageScript, Workspace, WorkspaceKind,
};
use langbank::EcosystemId;

#[derive(Debug, Default, Deserialize)]
struct NodeManifest {
    name: Option<String>,
    private: Option<bool>,
    workspaces: Option<NodeWorkspaces>,
    #[serde(rename = "packageManager")]
    package_manager: Option<String>,
    dependencies: Option<BTreeMap<String, Value>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<BTreeMap<String, Value>>,
    #[serde(rename = "peerDependencies")]
    peer_dependencies: Option<BTreeMap<String, Value>>,
    #[serde(rename = "optionalDependencies")]
    optional_dependencies: Option<BTreeMap<String, Value>>,
    scripts: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NodeWorkspaces {
    List(Vec<String>),
    Object { packages: Vec<String> },
}

impl NodeWorkspaces {
    fn patterns(self) -> Vec<String> {
        match self {
            Self::List(patterns) | Self::Object { packages: patterns } => patterns,
        }
    }
}

fn discover(builder: &mut DiscoveryBuilder<'_>) {
    let file_paths = builder.file_paths();
    for path in builder.manifest_paths("package.json") {
        builder.add_manifest_facts(parse(builder.root(), &file_paths, &path));
    }
}

fn resolve_ecosystems(builder: &mut DiscoveryBuilder<'_>) {
    let file_paths = builder.file_paths();
    let mut diagnostics = Vec::new();
    for package in builder
        .draft
        .packages
        .iter_mut()
        .filter(|package| package.kind == PackageKind::Node)
    {
        // A package only inherits manager evidence from a workspace it is
        // actually a member of. Directory ancestry alone is not ownership.
        let found = managers_in(&package.root, &file_paths);
        if found.len() > 1 {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Ecosystem,
                path: package.manifest.clone(),
                message: format!(
                    "conflicting package-manager lockfiles for {}",
                    found
                        .iter()
                        .map(EcosystemId::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }
        let from_lockfile = found.iter().next().cloned();
        if let (Some(declared), Some(selected)) =
            (package.ecosystem.as_ref(), from_lockfile.as_ref())
            && declared != selected
        {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Ecosystem,
                path: package.manifest.clone(),
                message: format!(
                    "packageManager declares {declared} but the lockfile selects {selected}"
                ),
            });
        }
        package.ecosystems.extend(found);
        package.ecosystem = from_lockfile.or_else(|| package.ecosystem.clone());
        if package.ecosystem.is_none() {
            package.ecosystem = Some(EcosystemId::from("npm"));
            package.ecosystems.insert(EcosystemId::from("npm"));
        }
    }
    builder.draft.diagnostics.extend(diagnostics);
}

fn parse(root: &Path, file_paths: &BTreeSet<PathBuf>, path: &Path) -> ManifestFacts {
    let mut facts = ManifestFacts {
        manifest: Manifest {
            path: path.to_path_buf(),
            kind: ManifestKind::PackageJson,
        },
        artifacts: Vec::new(),
        packages: Vec::new(),
        workspaces: Vec::new(),
        workspace_specs: Vec::new(),
        explicit_workspaces: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    // Which of the two went wrong is the whole diagnostic: a missing file and
    // a syntax error need different answers from whoever reads this.
    let parsed = std::fs::read_to_string(root.join(path))
        .map_err(|error| format!("package.json is unreadable: {error}"))
        .and_then(|text| {
            serde_json::from_str::<NodeManifest>(&text)
                .map_err(|error| format!("package.json is invalid JSON: {error}"))
        });
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(message) => {
            facts.diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Manifest,
                path: path.to_path_buf(),
                message,
            });
            return facts;
        }
    };

    let package_root = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
    let mut dependencies = Vec::new();
    collect_dependencies(
        &mut dependencies,
        parsed.dependencies,
        DependencyKind::Runtime,
    );
    collect_dependencies(
        &mut dependencies,
        parsed.dev_dependencies,
        DependencyKind::Development,
    );
    collect_dependencies(
        &mut dependencies,
        parsed.peer_dependencies,
        DependencyKind::Peer,
    );
    collect_dependencies(
        &mut dependencies,
        parsed.optional_dependencies,
        DependencyKind::Optional,
    );
    dependencies.sort();
    dependencies.dedup();
    let scripts = parsed
        .scripts
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, command)| {
            command.as_str().map(|command| PackageScript {
                name,
                command: command.to_owned(),
            })
        })
        .collect();
    let declaration = parsed.package_manager.as_deref();
    let selected = declaration
        .and_then(package_manager_id)
        .map(EcosystemId::from);
    if let Some(declaration) = declaration
        && selected.is_none()
    {
        facts.diagnostics.push(Diagnostic {
            kind: DiagnosticKind::Ecosystem,
            path: path.to_path_buf(),
            message: format!("unsupported packageManager declaration {declaration:?}"),
        });
    }
    let ecosystems = selected.clone().into_iter().collect();
    facts.packages.push(Package {
        id: package_id(PackageKind::Node, &package_root),
        kind: PackageKind::Node,
        root: package_root.clone(),
        manifest: path.to_path_buf(),
        name: parsed.name,
        private: parsed.private,
        ecosystem: selected,
        ecosystems,
        languages: Vec::new(),
        dependencies,
        scripts,
        workspace: None,
        lockfile_owner: package_root.clone(),
        lockfile: None,
        evidence: BTreeSet::from([path.to_path_buf()]),
    });

    let mut patterns = parsed
        .workspaces
        .map(NodeWorkspaces::patterns)
        .unwrap_or_default();
    let pnpm_workspace = package_root.join("pnpm-workspace.yaml");
    if file_paths.contains(&pnpm_workspace) {
        match read_pnpm_workspace(&root.join(&pnpm_workspace)) {
            Ok(found) => patterns.extend(found),
            Err(message) => facts.diagnostics.push(Diagnostic {
                kind: DiagnosticKind::Workspace,
                path: pnpm_workspace,
                message,
            }),
        }
    }
    if !patterns.is_empty() {
        let id = workspace_id(WorkspaceKind::Node, &package_root);
        let mut includes = Vec::new();
        let mut excludes = Vec::new();
        for pattern in patterns {
            if let Some(pattern) = pattern.strip_prefix('!') {
                excludes.push(pattern.to_owned());
            } else {
                includes.push(pattern);
            }
        }
        facts.workspaces.push(Workspace {
            id: id.clone(),
            kind: WorkspaceKind::Node,
            root: package_root.clone(),
            manifest: path.to_path_buf(),
            declared_members: includes.clone(),
            excluded_members: excludes.clone(),
            members: Vec::new(),
        });
        facts.workspace_specs.push(WorkspaceSpec {
            id,
            kind: WorkspaceKind::Node,
            root: package_root,
            manifest: path.to_path_buf(),
            includes,
            excludes,
        });
    }
    facts
}

fn read_pnpm_workspace(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("pnpm workspace is unreadable: {error}"))?;
    let value = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text)
        .map_err(|error| format!("pnpm workspace is invalid YAML: {error}"))?;
    Ok(value
        .get("packages")
        .and_then(serde_yaml_ng::Value::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(serde_yaml_ng::Value::as_str)
        .map(ToOwned::to_owned)
        .collect())
}

fn collect_dependencies(
    out: &mut Vec<Dependency>,
    dependencies: Option<BTreeMap<String, Value>>,
    kind: DependencyKind,
) {
    out.extend(
        dependencies
            .unwrap_or_default()
            .into_iter()
            .map(|(name, value)| {
                let requirement = value.as_str().map(ToOwned::to_owned);
                let source = if requirement.as_deref().is_some_and(node_local_requirement) {
                    DependencySource::LocalPath
                } else if requirement.is_some() {
                    DependencySource::Registry
                } else {
                    DependencySource::Unknown
                };
                Dependency {
                    name,
                    package: None,
                    kind,
                    source,
                    requirement,
                }
            }),
    );
}

fn node_local_requirement(requirement: &str) -> bool {
    ["file:", "link:", "workspace:", "portal:", ".", "/"]
        .iter()
        .any(|prefix| requirement.starts_with(prefix))
}

fn package_manager_id(value: &str) -> Option<&'static str> {
    let name = value.split('@').next().unwrap_or(value);
    match name {
        "bun" => Some("bun"),
        "pnpm" => Some("pnpm"),
        "yarn" => Some("yarn"),
        "npm" => Some("npm"),
        _ => None,
    }
}

pub(super) fn managers_in(directory: &Path, files: &BTreeSet<PathBuf>) -> BTreeSet<EcosystemId> {
    [
        ("bun.lock", "bun"),
        ("bun.lockb", "bun"),
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("npm-shrinkwrap.json", "npm"),
        ("pnpm-workspace.yaml", "pnpm"),
    ]
    .into_iter()
    .filter(|(lockfile, _)| files.contains(&directory.join(lockfile)))
    .map(|(_, ecosystem)| EcosystemId::from(ecosystem))
    .collect()
}

static MANIFEST_HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "entl.node-manifests",
    phase: DiscoveryPhase::Manifests,
    run: discover,
};

static ECOSYSTEM_HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "entl.node-ecosystems",
    phase: DiscoveryPhase::Relationships,
    run: resolve_ecosystems,
};

registry::submit! { DiscoveryHandlerRegistration(&MANIFEST_HANDLER) }
registry::submit! { DiscoveryHandlerRegistration(&ECOSYSTEM_HANDLER) }
