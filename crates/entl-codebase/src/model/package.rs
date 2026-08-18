use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{PackageId, WorkspaceId};
use langbank::{EcosystemId, LanguageId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManifestKind {
    Cargo,
    PackageJson,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub path: PathBuf,
    pub kind: ManifestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    Cargo,
    Node,
}

impl PackageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Node => "node",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyKind {
    Runtime,
    Development,
    Build,
    Peer,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencySource {
    Registry,
    Git,
    LocalPath,
    Workspace,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Dependency {
    /// Name used by the declaring package.
    pub name: String,
    /// Upstream package name when the declaration is renamed.
    pub package: Option<String>,
    pub kind: DependencyKind,
    pub source: DependencySource,
    pub requirement: Option<String>,
}

impl Dependency {
    pub fn package_name(&self) -> &str {
        self.package.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyResolution {
    pub ecosystem: EcosystemId,
    pub lockfile: PathBuf,
    pub packages: Vec<ResolvedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageLanguage {
    pub language: LanguageId,
    /// Codebase-relative files that establish the language.
    pub evidence: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PackageScript {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub id: PackageId,
    pub kind: PackageKind,
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub name: Option<String>,
    pub private: Option<bool>,
    /// The selected package manager or tool. `ecosystems` retains every
    /// observed ecosystem when the evidence conflicts.
    pub ecosystem: Option<EcosystemId>,
    pub ecosystems: BTreeSet<EcosystemId>,
    pub languages: Vec<PackageLanguage>,
    pub dependencies: Vec<Dependency>,
    pub scripts: Vec<PackageScript>,
    pub workspace: Option<WorkspaceId>,
    pub lockfile_owner: PathBuf,
    pub lockfile: Option<PathBuf>,
    pub evidence: BTreeSet<PathBuf>,
}

impl Package {
    pub fn has_language(&self, language: &str) -> bool {
        self.languages
            .iter()
            .any(|candidate| candidate.language.as_str() == language)
    }

    pub fn uses_ecosystem(&self, ecosystem: &str) -> bool {
        self.ecosystems
            .iter()
            .any(|candidate| candidate.as_str() == ecosystem)
    }

    pub fn depends_on(&self, package: &str) -> bool {
        self.dependencies
            .iter()
            .any(|dependency| dependency.package_name() == package)
    }

    pub fn script(&self, name: &str) -> Option<&PackageScript> {
        self.scripts
            .binary_search_by_key(&name, |script| script.name.as_str())
            .ok()
            .map(|index| &self.scripts[index])
    }
}
