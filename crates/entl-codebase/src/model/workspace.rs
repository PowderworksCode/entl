use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{PackageId, PackageKind, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceKind {
    Cargo,
    Node,
}

impl WorkspaceKind {
    pub(crate) fn package_kind(self) -> PackageKind {
        match self {
            Self::Cargo => PackageKind::Cargo,
            Self::Node => PackageKind::Node,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Node => "node",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub kind: WorkspaceKind,
    pub root: PathBuf,
    pub manifest: PathBuf,
    pub declared_members: Vec<String>,
    pub excluded_members: Vec<String>,
    pub members: Vec<PackageId>,
}
