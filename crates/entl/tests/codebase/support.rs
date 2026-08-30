// Fixtures shared by the mirrored codebase tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(dead_code)]

pub use entl::codebase::{
    DiagnosticKind, EcosystemId, InventoryOptions, LanguageEvidence, PackageId, SHELL_LANGUAGE,
    WorkspaceId, inspect, walk,
};
pub use std::collections::BTreeSet;
pub use std::fs;
pub use std::path::Path;

pub fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}
