use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;
use toml::Value;

use super::{
    DiscoveryBuilder, DiscoveryHandler, DiscoveryHandlerRegistration, DiscoveryPhase,
    ManifestFacts, WorkspaceSpec, normalize_relative, package_id, registry, workspace_id,
};
use crate::{
    Artifact, BINARY_ARTIFACT, Dependency, DependencyKind, Diagnostic, DiagnosticKind, EcosystemId,
    Manifest, ManifestKind, Package, PackageKind, Workspace, WorkspaceKind,
};

#[derive(Debug, Default, Deserialize)]
struct CargoManifest {
    package: Option<CargoPackage>,
    workspace: Option<CargoWorkspace>,
    dependencies: Option<BTreeMap<String, Value>>,
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: Option<BTreeMap<String, Value>>,
    #[serde(rename = "build-dependencies")]
    build_dependencies: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    bin: Vec<CargoTarget>,
}

#[derive(Debug, Default, Deserialize)]
struct CargoTarget {}

#[derive(Debug, Default, Deserialize)]
struct CargoPackage {
    name: Option<String>,
    workspace: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CargoWorkspace {
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

fn discover(builder: &mut DiscoveryBuilder<'_>) {
    for path in builder.manifest_paths("Cargo.toml") {
        builder.add_manifest_facts(parse(builder.root(), &path));
    }
}

fn parse(root: &Path, path: &Path) -> ManifestFacts {
    let mut facts = ManifestFacts {
        manifest: Manifest {
            path: path.to_path_buf(),
            kind: ManifestKind::Cargo,
        },
        artifacts: Vec::new(),
        packages: Vec::new(),
        workspaces: Vec::new(),
        workspace_specs: Vec::new(),
        explicit_workspaces: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    let parsed = std::fs::read_to_string(root.join(path))
        .ok()
        .and_then(|text| toml::from_str::<CargoManifest>(&text).ok());
    let Some(parsed) = parsed else {
        facts.diagnostics.push(Diagnostic {
            kind: DiagnosticKind::Manifest,
            path: path.to_path_buf(),
            message: "Cargo manifest is unreadable or invalid TOML".to_owned(),
        });
        return facts;
    };

    let package_root = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
    if !parsed.bin.is_empty() {
        facts.artifacts.push(Artifact {
            profile: (&BINARY_ARTIFACT).into(),
            root: package_root.clone(),
            evidence: BTreeSet::from([path.to_path_buf()]),
        });
    }
    if let Some(package) = parsed.package {
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
            parsed.build_dependencies,
            DependencyKind::Build,
        );
        dependencies.sort();
        dependencies.dedup();
        let id = package_id(PackageKind::Cargo, &package_root);
        if let Some(workspace) = package.workspace {
            facts.explicit_workspaces.insert(
                id.clone(),
                normalize_relative(&package_root, Path::new(&workspace)),
            );
        }
        facts.packages.push(Package {
            id,
            kind: PackageKind::Cargo,
            root: package_root.clone(),
            manifest: path.to_path_buf(),
            name: package.name,
            private: None,
            ecosystem: Some(EcosystemId::from("cargo")),
            ecosystems: BTreeSet::from([EcosystemId::from("cargo")]),
            languages: Vec::new(),
            dependencies,
            scripts: Vec::new(),
            workspace: None,
            lockfile_owner: package_root.clone(),
            lockfile: None,
            evidence: BTreeSet::from([path.to_path_buf()]),
        });
    }
    if let Some(workspace) = parsed.workspace {
        let id = workspace_id(WorkspaceKind::Cargo, &package_root);
        let declared_members = workspace.members;
        let excluded_members = workspace.exclude;
        facts.workspaces.push(Workspace {
            id: id.clone(),
            kind: WorkspaceKind::Cargo,
            root: package_root.clone(),
            manifest: path.to_path_buf(),
            declared_members: declared_members.clone(),
            excluded_members: excluded_members.clone(),
            members: Vec::new(),
        });
        facts.workspace_specs.push(WorkspaceSpec {
            id,
            kind: WorkspaceKind::Cargo,
            root: package_root,
            manifest: path.to_path_buf(),
            includes: declared_members,
            excludes: excluded_members,
        });
    }
    facts
}

fn collect_dependencies(
    out: &mut Vec<Dependency>,
    dependencies: Option<BTreeMap<String, Value>>,
    kind: DependencyKind,
) {
    out.extend(
        dependencies
            .unwrap_or_default()
            .into_keys()
            .map(|name| Dependency { name, kind }),
    );
}

static HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "entl.cargo-manifests",
    phase: DiscoveryPhase::Manifests,
    run: discover,
};

registry::submit! { DiscoveryHandlerRegistration(&HANDLER) }
