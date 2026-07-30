use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    Artifact, ArtifactId, Diagnostic, FileEntry, Manifest, Package, PackageId, Project, Workspace,
    WorkspaceId,
};
use crate::LanguageProfile;
use crate::{Error, Result};

/// The file-level result of walking a local codebase, before manifest and
/// package analysis. Paths are root-relative and content remains lazy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodebaseTree {
    /// Canonical absolute root used for lazy content reads.
    pub root: PathBuf,
    pub files: Vec<FileEntry>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CodebaseTree {
    pub fn file(&self, path: impl AsRef<Path>) -> Option<&FileEntry> {
        let path = path.as_ref();
        self.files.iter().find(|file| file.path == path)
    }

    pub fn files_with_language<'a>(
        &'a self,
        language: &'a str,
    ) -> impl Iterator<Item = &'a FileEntry> + 'a {
        self.files
            .iter()
            .filter(move |file| file.has_language(language))
    }

    pub fn files_with_language_profile<'a>(
        &'a self,
        language: &'a LanguageProfile,
    ) -> impl Iterator<Item = &'a FileEntry> + 'a {
        self.files
            .iter()
            .filter(move |file| file.has_language_profile(language))
    }

    pub fn has_file(&self, path: impl AsRef<Path>) -> bool {
        self.file(path).is_some()
    }

    pub fn read_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        read_bytes(&self.root, path.as_ref())
    }

    pub fn read_text(&self, path: impl AsRef<Path>) -> Result<String> {
        read_text(&self.root, path.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodebaseInventory {
    /// Canonical absolute root used for lazy content reads. All other paths are
    /// root-relative and deterministic.
    pub root: PathBuf,
    pub files: Vec<FileEntry>,
    pub manifests: Vec<Manifest>,
    pub artifacts: Vec<Artifact>,
    pub projects: Vec<Project>,
    pub packages: Vec<Package>,
    pub workspaces: Vec<Workspace>,
    pub diagnostics: Vec<Diagnostic>,
}

impl CodebaseInventory {
    pub fn file(&self, path: impl AsRef<Path>) -> Option<&FileEntry> {
        let path = path.as_ref();
        self.files.iter().find(|file| file.path == path)
    }

    pub fn package(&self, id: &PackageId) -> Option<&Package> {
        self.packages.iter().find(|package| &package.id == id)
    }

    pub fn artifacts_at(&self, root: impl AsRef<Path>) -> impl Iterator<Item = &Artifact> {
        let root = root.as_ref().to_path_buf();
        self.artifacts
            .iter()
            .filter(move |artifact| artifact.root == root)
    }

    pub fn artifacts_with_profile<'a>(
        &'a self,
        profile: &'a ArtifactId,
    ) -> impl Iterator<Item = &'a Artifact> + 'a {
        self.artifacts
            .iter()
            .filter(move |artifact| &artifact.profile == profile)
    }

    pub fn project(&self, root: impl AsRef<Path>) -> Option<&Project> {
        let root = root.as_ref();
        self.projects.iter().find(|project| project.root == root)
    }

    pub fn workspace(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|workspace| &workspace.id == id)
    }

    pub fn files_for_package<'a>(
        &'a self,
        id: &'a PackageId,
    ) -> impl Iterator<Item = &'a FileEntry> + 'a {
        self.files
            .iter()
            .filter(move |file| file.packages.contains(id))
    }

    pub fn files_with_language<'a>(
        &'a self,
        language: &'a str,
    ) -> impl Iterator<Item = &'a FileEntry> + 'a {
        self.files
            .iter()
            .filter(move |file| file.has_language(language))
    }

    pub fn files_with_language_profile<'a>(
        &'a self,
        language: &'a LanguageProfile,
    ) -> impl Iterator<Item = &'a FileEntry> + 'a {
        self.files
            .iter()
            .filter(move |file| file.has_language_profile(language))
    }

    pub fn packages_at(&self, root: impl AsRef<Path>) -> impl Iterator<Item = &Package> {
        let root = root.as_ref().to_path_buf();
        self.packages
            .iter()
            .filter(move |package| package.root == root)
    }

    pub fn package_owners(&self, path: impl AsRef<Path>) -> Vec<&Package> {
        self.file(path)
            .into_iter()
            .flat_map(|file| &file.packages)
            .filter_map(|id| self.package(id))
            .collect()
    }

    pub fn has_file(&self, path: impl AsRef<Path>) -> bool {
        self.file(path).is_some()
    }

    pub fn read_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        read_bytes(&self.root, path.as_ref())
    }

    pub fn read_text(&self, path: impl AsRef<Path>) -> Result<String> {
        read_text(&self.root, path.as_ref())
    }
}

fn read_bytes(root: &Path, path: &Path) -> Result<Vec<u8>> {
    let relative = safe_relative(path)?;
    let absolute = root.join(relative);
    std::fs::read(&absolute).map_err(|source| Error::Read {
        path: absolute,
        source,
    })
}

fn read_text(root: &Path, path: &Path) -> Result<String> {
    let relative = safe_relative(path)?;
    let bytes = read_bytes(root, relative)?;
    String::from_utf8(bytes).map_err(|_| Error::NonUtf8 {
        path: root.join(relative),
    })
}

fn safe_relative(path: &Path) -> Result<&Path> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(Error::UnsafePath(path.to_path_buf()));
    }
    Ok(path)
}
