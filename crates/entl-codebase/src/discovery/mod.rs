use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use globset::{Glob, GlobMatcher};
mod cargo;
mod node;

use crate::{
    Artifact, ArtifactProfile, BINARY_ARTIFACT, CodebaseInventory, Diagnostic, DiagnosticKind,
    EcosystemId, FileEntry, InventoryOptions, LanguageId, Manifest, Package, PackageId,
    PackageKind, PackageLanguage, Project, ProjectFacetId, Result, Workspace, WorkspaceId,
    WorkspaceKind, artifact_profile, artifact_profiles, walk,
};
use crate::{LanguageRole, ecosystem_profile, language_profile, language_profiles};

pub use registry_inventory as registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveryPhase {
    Manifests,
    Relationships,
    Projects,
    Enrichment,
}

pub type DiscoveryRunner = for<'a> fn(&mut DiscoveryBuilder<'a>);

pub struct DiscoveryHandler {
    pub id: &'static str,
    pub phase: DiscoveryPhase,
    pub run: DiscoveryRunner,
}

pub struct DiscoveryHandlerRegistration(pub &'static DiscoveryHandler);

registry::collect!(DiscoveryHandlerRegistration);

static HANDLERS: LazyLock<Vec<&'static DiscoveryHandler>> = LazyLock::new(|| {
    let mut handlers = registry::iter::<DiscoveryHandlerRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    let mut ids = BTreeSet::new();
    for handler in &handlers {
        assert!(ids.insert(handler.id), "duplicate discovery handler ID");
    }
    handlers.sort_by_key(|handler| (handler.phase, handler.id));
    handlers
});

pub fn discovery_handlers() -> &'static [&'static DiscoveryHandler] {
    &HANDLERS
}

/// Inspect one local working tree. The root is canonicalized once; every fact
/// below it uses a codebase-relative path. Local failures become diagnostics
/// wherever a useful partial inventory can still be returned.
pub fn inspect(root: impl AsRef<Path>, options: &InventoryOptions) -> Result<CodebaseInventory> {
    let tree = walk(root, options)?;
    let crate::CodebaseTree {
        root,
        files,
        diagnostics,
    } = tree;

    let mut builder = DiscoveryBuilder::new(&root, files, diagnostics);
    for handler in discovery_handlers() {
        (handler.run)(&mut builder);
    }
    builder.finish()
}

pub struct DiscoveryBuilder<'a> {
    draft: InventoryDraft<'a>,
}

impl<'a> DiscoveryBuilder<'a> {
    fn new(root: &'a Path, files: Vec<FileEntry>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            draft: InventoryDraft {
                root,
                files,
                manifests: Vec::new(),
                artifacts: Vec::new(),
                projects: Vec::new(),
                packages: Vec::new(),
                workspaces: Vec::new(),
                workspace_specs: Vec::new(),
                explicit_workspaces: BTreeMap::new(),
                diagnostics,
            },
        }
    }

    pub fn root(&self) -> &Path {
        self.draft.root
    }

    pub fn files(&self) -> &[FileEntry] {
        &self.draft.files
    }

    pub fn packages(&self) -> &[Package] {
        &self.draft.packages
    }

    pub fn projects(&self) -> &[Project] {
        &self.draft.projects
    }

    pub fn add_project_language(
        &mut self,
        root: impl Into<PathBuf>,
        language: impl Into<LanguageId>,
        evidence: impl IntoIterator<Item = PathBuf>,
    ) {
        let root = root.into();
        let project = self.project_mut(root);
        let language = language.into();
        let evidence = evidence.into_iter().collect::<BTreeSet<_>>();
        project.evidence.extend(evidence.iter().cloned());
        if let Some(existing) = project
            .languages
            .iter_mut()
            .find(|candidate| candidate.language == language)
        {
            existing.evidence.extend(evidence);
            existing.evidence.sort();
            existing.evidence.dedup();
        } else {
            project.languages.push(PackageLanguage {
                language,
                evidence: evidence.into_iter().collect(),
            });
            project
                .languages
                .sort_by(|left, right| left.language.cmp(&right.language));
        }
    }

    pub fn add_project_facet(
        &mut self,
        root: impl Into<PathBuf>,
        facet: impl Into<ProjectFacetId>,
        evidence: impl IntoIterator<Item = PathBuf>,
    ) {
        let project = self.project_mut(root.into());
        project.facets.insert(facet.into());
        project.evidence.extend(evidence);
    }

    pub fn add_artifact(
        &mut self,
        profile: &'static ArtifactProfile,
        root: impl Into<PathBuf>,
        evidence: impl IntoIterator<Item = PathBuf>,
    ) {
        self.draft.artifacts.push(Artifact {
            profile: profile.into(),
            root: root.into(),
            evidence: evidence.into_iter().collect(),
        });
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.draft.diagnostics.push(diagnostic);
    }

    fn project_mut(&mut self, root: PathBuf) -> &mut Project {
        if let Some(index) = self
            .draft
            .projects
            .iter()
            .position(|project| project.root == root)
        {
            return &mut self.draft.projects[index];
        }
        self.draft.projects.push(Project {
            root,
            packages: Vec::new(),
            languages: Vec::new(),
            ecosystems: BTreeSet::new(),
            facets: BTreeSet::new(),
            evidence: BTreeSet::new(),
        });
        self.draft
            .projects
            .last_mut()
            .expect("project was inserted")
    }

    fn finish(self) -> Result<CodebaseInventory> {
        self.draft.finish()
    }

    fn manifest_paths(&self, filename: &str) -> Vec<PathBuf> {
        self.draft
            .files
            .iter()
            .filter(|file| file.path.file_name().and_then(|name| name.to_str()) == Some(filename))
            .map(|file| file.path.clone())
            .collect()
    }

    fn file_paths(&self) -> BTreeSet<PathBuf> {
        self.draft
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect()
    }

    fn add_manifest_facts(&mut self, facts: ManifestFacts) {
        self.draft.manifests.push(facts.manifest);
        self.draft.artifacts.extend(facts.artifacts);
        self.draft.packages.extend(facts.packages);
        self.draft.workspaces.extend(facts.workspaces);
        self.draft.workspace_specs.extend(facts.workspace_specs);
        self.draft
            .explicit_workspaces
            .extend(facts.explicit_workspaces);
        self.draft.diagnostics.extend(facts.diagnostics);
    }
}

fn resolve_relationships(builder: &mut DiscoveryBuilder<'_>) {
    builder.draft.resolve_workspaces();
    builder.draft.resolve_package_lockfiles();
    builder.draft.assign_files();
    builder.draft.resolve_package_languages();
}

fn resolve_projects(builder: &mut DiscoveryBuilder<'_>) {
    builder.draft.resolve_projects();
}

fn resolve_artifacts(builder: &mut DiscoveryBuilder<'_>) {
    builder.draft.resolve_artifacts();
}

static RELATIONSHIP_HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "entl.relationships",
    phase: DiscoveryPhase::Relationships,
    run: resolve_relationships,
};

static PROJECT_HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "entl.projects",
    phase: DiscoveryPhase::Projects,
    run: resolve_projects,
};

static ARTIFACT_HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "entl.artifacts",
    phase: DiscoveryPhase::Enrichment,
    run: resolve_artifacts,
};

registry::submit! { DiscoveryHandlerRegistration(&RELATIONSHIP_HANDLER) }
registry::submit! { DiscoveryHandlerRegistration(&PROJECT_HANDLER) }
registry::submit! { DiscoveryHandlerRegistration(&ARTIFACT_HANDLER) }

struct ManifestFacts {
    manifest: Manifest,
    artifacts: Vec<Artifact>,
    packages: Vec<Package>,
    workspaces: Vec<Workspace>,
    workspace_specs: Vec<WorkspaceSpec>,
    explicit_workspaces: BTreeMap<PackageId, PathBuf>,
    diagnostics: Vec<Diagnostic>,
}

struct InventoryDraft<'a> {
    root: &'a Path,
    files: Vec<FileEntry>,
    manifests: Vec<Manifest>,
    artifacts: Vec<Artifact>,
    projects: Vec<Project>,
    packages: Vec<Package>,
    workspaces: Vec<Workspace>,
    workspace_specs: Vec<WorkspaceSpec>,
    explicit_workspaces: BTreeMap<PackageId, PathBuf>,
    diagnostics: Vec<Diagnostic>,
}

impl InventoryDraft<'_> {
    fn resolve_artifacts(&mut self) {
        for project in &self.projects {
            for profile in artifact_profiles() {
                if profile
                    .project_facets
                    .iter()
                    .any(|facet| project.has_facet(facet))
                {
                    self.artifacts.push(Artifact {
                        profile: (*profile).into(),
                        root: project.root.clone(),
                        evidence: project.evidence.clone(),
                    });
                }
            }
        }
        for package in &self.packages {
            for profile in artifact_profiles() {
                let dependency = profile
                    .package_dependencies
                    .iter()
                    .any(|dependency| package.depends_on(dependency));
                let script = package.scripts.iter().any(|script| {
                    profile
                        .package_script_signals
                        .iter()
                        .any(|signal| script.command.contains(signal))
                });
                if dependency || script {
                    self.artifacts.push(Artifact {
                        profile: (*profile).into(),
                        root: package.root.clone(),
                        evidence: BTreeSet::from([package.manifest.clone()]),
                    });
                }
            }
            let tauri_backend = self.projects.iter().any(|project| {
                project.has_facet("tauri") && package.root == project.root.join("src-tauri")
            });
            if package.kind == PackageKind::Cargo && !tauri_backend {
                let main = package.root.join("src/main.rs");
                let bin = package.root.join("src/bin");
                let marker = self.files.iter().find(|file| {
                    file.path == main
                        || (file.path.starts_with(&bin)
                            && file
                                .path
                                .extension()
                                .and_then(|extension| extension.to_str())
                                == Some("rs"))
                });
                if let Some(marker) = marker {
                    self.artifacts.push(Artifact {
                        profile: (&BINARY_ARTIFACT).into(),
                        root: package.root.clone(),
                        evidence: BTreeSet::from([package.manifest.clone(), marker.path.clone()]),
                    });
                }
            }
        }
    }

    fn resolve_package_lockfiles(&mut self) {
        let files = self
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        let workspace_roots = self
            .workspaces
            .iter()
            .map(|workspace| (workspace.id.clone(), workspace.root.clone()))
            .collect::<BTreeMap<_, _>>();
        let package_ecosystems = self
            .packages
            .iter()
            .map(|package| {
                (
                    (package.kind, package.root.clone()),
                    package.ecosystem.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        for package in &mut self.packages {
            let owner = package
                .workspace
                .as_ref()
                .and_then(|workspace| workspace_roots.get(workspace))
                .cloned()
                .unwrap_or_else(|| package.root.clone());
            package.lockfile_owner = owner.clone();
            if package.kind == PackageKind::Node
                && owner != package.root
                && let Some(Some(ecosystem)) =
                    package_ecosystems.get(&(PackageKind::Node, owner.clone()))
            {
                package.ecosystem = Some(ecosystem.clone());
                package.ecosystems.insert(ecosystem.clone());
            }
            let Some(profile) = package
                .ecosystem
                .as_ref()
                .and_then(|ecosystem| ecosystem_profile(ecosystem.as_str()))
            else {
                continue;
            };
            package.lockfile = profile
                .lockfiles
                .iter()
                .map(|lockfile| owner.join(lockfile))
                .find(|path| files.contains(path));
            if let Some(lockfile) = &package.lockfile {
                package.evidence.insert(lockfile.clone());
            }
        }
    }

    fn resolve_workspaces(&mut self) {
        let rules: Vec<CompiledWorkspace> = self
            .workspace_specs
            .iter()
            .filter_map(|spec| match CompiledWorkspace::new(spec) {
                Ok(rule) => Some(rule),
                Err((pattern, message)) => {
                    self.diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Workspace,
                        path: spec.manifest.clone(),
                        message: format!("invalid workspace pattern {pattern:?}: {message}"),
                    });
                    None
                }
            })
            .collect();

        for package in &mut self.packages {
            if let Some(explicit) = self.explicit_workspaces.get(&package.id) {
                if let Some(workspace) = self.workspaces.iter().find(|workspace| {
                    workspace.kind == WorkspaceKind::Cargo && workspace.root == *explicit
                }) {
                    package.workspace = Some(workspace.id.clone());
                } else {
                    self.diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Workspace,
                        path: package.manifest.clone(),
                        message: format!(
                            "package declares workspace {} but no workspace manifest was found there",
                            explicit.display()
                        ),
                    });
                }
                continue;
            }
            let mut candidates: Vec<&CompiledWorkspace> = rules
                .iter()
                .filter(|workspace| workspace.matches(package))
                .collect();
            candidates.sort_by_key(|workspace| workspace.root.components().count());
            if let Some(workspace) = candidates.last() {
                package.workspace = Some(workspace.id.clone());
            }
        }

        let members_by_workspace: BTreeMap<WorkspaceId, Vec<PackageId>> = self
            .packages
            .iter()
            .filter_map(|package| {
                package
                    .workspace
                    .as_ref()
                    .map(|workspace| (workspace.clone(), package.id.clone()))
            })
            .fold(BTreeMap::new(), |mut map, (workspace, package)| {
                map.entry(workspace).or_default().push(package);
                map
            });
        for workspace in &mut self.workspaces {
            workspace.members = members_by_workspace
                .get(&workspace.id)
                .cloned()
                .unwrap_or_default();
            workspace.members.sort();
        }

        for rule in &rules {
            for (pattern, matcher) in &rule.includes {
                let matched = self.packages.iter().any(|package| {
                    package.kind == rule.kind.package_kind()
                        && package.root.starts_with(&rule.root)
                        && package.root != rule.root
                        && matcher.is_match(path_key(
                            package
                                .root
                                .strip_prefix(&rule.root)
                                .unwrap_or(&package.root),
                        ))
                });
                if !matched {
                    self.diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::Workspace,
                        path: rule.manifest.clone(),
                        message: format!("workspace member pattern {pattern:?} matched no package"),
                    });
                }
            }
        }
    }

    fn assign_files(&mut self) {
        for file in &mut self.files {
            for kind in [PackageKind::Cargo, PackageKind::Node] {
                let owner = self
                    .packages
                    .iter()
                    .filter(|package| package.kind == kind && file.path.starts_with(&package.root))
                    .max_by_key(|package| package.root.components().count());
                if let Some(owner) = owner {
                    file.packages.push(owner.id.clone());
                }
            }
            file.packages.sort();
        }
    }

    fn resolve_package_languages(&mut self) {
        let mut evidence: BTreeMap<PackageId, BTreeMap<LanguageId, BTreeSet<PathBuf>>> =
            BTreeMap::new();
        for file in &self.files {
            let Some(detection) = &file.language else {
                continue;
            };
            let Some(profile) = language_profile(detection.language.as_str()) else {
                continue;
            };
            if !matches!(
                profile.role,
                LanguageRole::Programming | LanguageRole::Markup | LanguageRole::Stylesheet
            ) {
                continue;
            }
            for package in &file.packages {
                evidence
                    .entry(package.clone())
                    .or_default()
                    .entry(detection.language.clone())
                    .or_default()
                    .insert(file.path.clone());
            }
        }
        for package in &self.packages {
            if let Some(ecosystem) = package
                .ecosystem
                .as_ref()
                .and_then(|ecosystem| ecosystem_profile(ecosystem.as_str()))
            {
                for language in ecosystem.implied_languages {
                    evidence
                        .entry(package.id.clone())
                        .or_default()
                        .entry(LanguageId::from(*language))
                        .or_default()
                        .insert(package.manifest.clone());
                }
            }
            let package_files = self
                .files
                .iter()
                .filter_map(|file| {
                    (file.path.parent().unwrap_or_else(|| Path::new("")) == package.root)
                        .then(|| file.path.file_name()?.to_str().map(ToOwned::to_owned))
                        .flatten()
                })
                .collect::<BTreeSet<_>>();
            let dependencies = package
                .dependencies
                .iter()
                .map(|dependency| dependency.name.clone())
                .collect::<BTreeSet<_>>();
            for language in language_profiles() {
                if !language.detects_project(&package_files, &dependencies) {
                    continue;
                }
                let language_evidence = language
                    .config_files
                    .iter()
                    .map(|config| package.root.join(config))
                    .find(|path| self.files.iter().any(|file| file.path == *path))
                    .unwrap_or_else(|| package.manifest.clone());
                evidence
                    .entry(package.id.clone())
                    .or_default()
                    .entry(LanguageId::from(*language))
                    .or_default()
                    .insert(language_evidence);
            }
        }
        for package in &mut self.packages {
            package.languages = evidence
                .remove(&package.id)
                .unwrap_or_default()
                .into_iter()
                .map(|(language, paths)| PackageLanguage {
                    language,
                    evidence: paths.into_iter().collect(),
                })
                .collect();
        }
    }

    fn resolve_projects(&mut self) {
        const WEB_PACKAGES: &[&str] = &["vite", "next", "astro", "@sveltejs/kit", "parcel"];
        const WEB_CONFIGS: &[&str] = &[
            "vite.config.ts",
            "vite.config.js",
            "next.config.ts",
            "next.config.js",
            "astro.config.ts",
            "astro.config.mjs",
            "svelte.config.js",
        ];
        const TAURI_CONFIGS: &[&str] = &["tauri.conf.json", "tauri.conf.json5"];

        let mut roots = self
            .packages
            .iter()
            .map(|package| package.root.clone())
            .collect::<BTreeSet<_>>();
        roots.extend(
            self.workspaces
                .iter()
                .map(|workspace| workspace.root.clone()),
        );
        for file in &self.files {
            let Some(name) = file.path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if language_profiles()
                .iter()
                .any(|profile| profile.config_files.contains(&name))
            {
                roots.insert(
                    file.path
                        .parent()
                        .unwrap_or_else(|| Path::new(""))
                        .to_path_buf(),
                );
            }
        }
        if roots.is_empty()
            && self.files.iter().any(|file| {
                file.language
                    .as_ref()
                    .and_then(|language| language_profile(language.language.as_str()))
                    .is_some_and(|profile| {
                        matches!(
                            profile.role,
                            LanguageRole::Programming
                                | LanguageRole::Markup
                                | LanguageRole::Stylesheet
                        )
                    })
            })
        {
            roots.insert(PathBuf::new());
        }

        let mut packages = BTreeMap::<PathBuf, Vec<PackageId>>::new();
        let mut languages = BTreeMap::<PathBuf, BTreeMap<LanguageId, BTreeSet<PathBuf>>>::new();
        let mut ecosystems = BTreeMap::<PathBuf, BTreeSet<EcosystemId>>::new();
        let mut facets = BTreeMap::<PathBuf, BTreeSet<ProjectFacetId>>::new();
        let mut evidence = BTreeMap::<PathBuf, BTreeSet<PathBuf>>::new();

        for workspace in &self.workspaces {
            evidence
                .entry(workspace.root.clone())
                .or_default()
                .insert(workspace.manifest.clone());
            if workspace.kind == WorkspaceKind::Cargo {
                facets
                    .entry(workspace.root.clone())
                    .or_default()
                    .insert(ProjectFacetId::from("cargo-workspace"));
            }
        }

        for package in &self.packages {
            packages
                .entry(package.root.clone())
                .or_default()
                .push(package.id.clone());
            ecosystems
                .entry(package.root.clone())
                .or_default()
                .extend(package.ecosystems.iter().cloned());
            evidence
                .entry(package.root.clone())
                .or_default()
                .extend(package.evidence.iter().cloned());
            for language in &package.languages {
                let owned_evidence = language
                    .evidence
                    .iter()
                    .filter(|path| {
                        roots
                            .iter()
                            .filter(|root| path.starts_with(root))
                            .max_by_key(|root| root.components().count())
                            .is_none_or(|owner| owner == &package.root)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if owned_evidence.is_empty() {
                    continue;
                }
                languages
                    .entry(package.root.clone())
                    .or_default()
                    .entry(language.language.clone())
                    .or_default()
                    .extend(owned_evidence);
            }
            let web_dependency = package
                .dependencies
                .iter()
                .any(|dependency| WEB_PACKAGES.contains(&dependency.name.as_str()));
            let web_config = WEB_CONFIGS.iter().any(|config| {
                self.files
                    .iter()
                    .any(|file| file.path == package.root.join(config))
            });
            if web_dependency || web_config {
                facets
                    .entry(package.root.clone())
                    .or_default()
                    .insert(ProjectFacetId::from("static-site"));
            }
            let tauri_config = TAURI_CONFIGS.iter().any(|config| {
                self.files
                    .iter()
                    .any(|file| file.path == package.root.join("src-tauri").join(config))
            });
            if tauri_config {
                facets
                    .entry(package.root.clone())
                    .or_default()
                    .insert(ProjectFacetId::from("tauri"));
            }
        }

        for file in &self.files {
            let Some(detection) = &file.language else {
                continue;
            };
            let Some(profile) = language_profile(detection.language.as_str()) else {
                continue;
            };
            if !matches!(
                profile.role,
                LanguageRole::Programming | LanguageRole::Markup | LanguageRole::Stylesheet
            ) {
                continue;
            }
            let owner = roots
                .iter()
                .filter(|root| file.path.starts_with(root))
                .max_by_key(|root| root.components().count())
                .cloned()
                .unwrap_or_default();
            roots.insert(owner.clone());
            languages
                .entry(owner.clone())
                .or_default()
                .entry(detection.language.clone())
                .or_default()
                .insert(file.path.clone());
            evidence.entry(owner).or_default().insert(file.path.clone());
        }

        self.projects = roots
            .into_iter()
            .map(|root| {
                let mut project_packages = packages.remove(&root).unwrap_or_default();
                project_packages.sort();
                Project {
                    root: root.clone(),
                    packages: project_packages,
                    languages: languages
                        .remove(&root)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(language, evidence)| PackageLanguage {
                            language,
                            evidence: evidence.into_iter().collect(),
                        })
                        .collect(),
                    ecosystems: ecosystems.remove(&root).unwrap_or_default(),
                    facets: facets.remove(&root).unwrap_or_default(),
                    evidence: evidence.remove(&root).unwrap_or_default(),
                }
            })
            .collect();
    }

    fn finish(mut self) -> Result<CodebaseInventory> {
        let mut artifacts = BTreeMap::new();
        for artifact in self.artifacts {
            assert!(
                artifact_profile(artifact.profile.as_str()).is_some(),
                "discovered artifact references an unregistered profile"
            );
            artifacts
                .entry((artifact.root, artifact.profile))
                .or_insert_with(BTreeSet::new)
                .extend(artifact.evidence);
        }
        self.artifacts = artifacts
            .into_iter()
            .map(|((root, profile), evidence)| Artifact {
                profile,
                root,
                evidence,
            })
            .collect();
        self.manifests
            .sort_by(|left, right| left.path.cmp(&right.path));
        self.packages.sort_by(|left, right| left.id.cmp(&right.id));
        self.projects
            .sort_by(|left, right| left.root.cmp(&right.root));
        self.workspaces
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.diagnostics.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.kind.cmp(&right.kind))
                .then(left.message.cmp(&right.message))
        });
        Ok(CodebaseInventory {
            root: self.root.to_path_buf(),
            files: self.files,
            manifests: self.manifests,
            artifacts: self.artifacts,
            projects: self.projects,
            packages: self.packages,
            workspaces: self.workspaces,
            diagnostics: self.diagnostics,
        })
    }
}

#[derive(Clone)]
struct WorkspaceSpec {
    id: WorkspaceId,
    kind: WorkspaceKind,
    root: PathBuf,
    manifest: PathBuf,
    includes: Vec<String>,
    excludes: Vec<String>,
}

struct CompiledWorkspace {
    id: WorkspaceId,
    kind: WorkspaceKind,
    root: PathBuf,
    manifest: PathBuf,
    includes: Vec<(String, GlobMatcher)>,
    excludes: Vec<GlobMatcher>,
}

impl CompiledWorkspace {
    fn new(spec: &WorkspaceSpec) -> std::result::Result<Self, (String, String)> {
        let mut includes = Vec::new();
        for pattern in &spec.includes {
            let matcher =
                workspace_glob(pattern).map_err(|error| (pattern.clone(), error.to_string()))?;
            includes.push((pattern.clone(), matcher));
        }
        let mut excludes = Vec::new();
        for pattern in &spec.excludes {
            let matcher =
                workspace_glob(pattern).map_err(|error| (pattern.clone(), error.to_string()))?;
            excludes.push(matcher);
        }
        Ok(Self {
            id: spec.id.clone(),
            kind: spec.kind,
            root: spec.root.clone(),
            manifest: spec.manifest.clone(),
            includes,
            excludes,
        })
    }

    fn matches(&self, package: &Package) -> bool {
        if package.kind != self.kind.package_kind() || !package.root.starts_with(&self.root) {
            return false;
        }
        if package.root == self.root {
            return true;
        }
        let relative = package
            .root
            .strip_prefix(&self.root)
            .unwrap_or(&package.root);
        let relative = path_key(relative);
        self.includes
            .iter()
            .any(|(_, matcher)| matcher.is_match(&relative))
            && !self
                .excludes
                .iter()
                .any(|matcher| matcher.is_match(&relative))
    }
}

fn workspace_glob(pattern: &str) -> std::result::Result<GlobMatcher, globset::Error> {
    Glob::new(pattern.trim_start_matches("./")).map(|glob| glob.compile_matcher())
}

fn package_id(kind: PackageKind, root: &Path) -> PackageId {
    PackageId::new(format!("{}:{}", kind.as_str(), path_label(root)))
}

fn workspace_id(kind: WorkspaceKind, root: &Path) -> WorkspaceId {
    WorkspaceId::new(format!("{}:{}", kind.as_str(), path_label(root)))
}

fn path_label(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        path_key(path)
    }
}

fn path_key(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_relative(base: &Path, path: &Path) -> PathBuf {
    let mut normalized = base.to_path_buf();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(component) => normalized.push(component),
            Component::RootDir | Component::Prefix(_) => return path.to_path_buf(),
        }
    }
    normalized
}
