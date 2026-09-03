// Fixtures shared by the mirrored GitHub inventory tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(dead_code)]

pub use entl::codebase::{
    BUN_ECOSYSTEM, CARGO_ECOSYSTEM, CODESPELL, HAWK, InventoryOptions, SHELLCHECK, TaskKind, VALE,
    ZIZMOR, inspect as inspect_codebase,
};
pub use entl::github::{dependabot_ecosystem_profile, inspect};
pub use std::fs;
pub use std::path::Path;
pub use std::path::PathBuf;

pub fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}
