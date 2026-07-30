use std::collections::BTreeSet;
use std::sync::LazyLock;

use super::registry;

/// A directory that can be pruned during a codebase walk.
///
/// When `markers` is non-empty, at least one marker must exist in the
/// directory's ancestry. This prevents ambiguous names such as `build` from
/// being treated as generated everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraversalDirectory {
    pub name: &'static str,
    pub markers: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
pub struct TraversalDirectoryRegistration(pub &'static TraversalDirectory);

registry::collect!(TraversalDirectoryRegistration);

static REGISTERED: LazyLock<Vec<&'static TraversalDirectory>> = LazyLock::new(|| {
    let mut directories = registry::iter::<TraversalDirectoryRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    directories.sort_by_key(|directory| (directory.name, directory.markers));
    let mut identities = BTreeSet::new();
    for directory in &directories {
        assert!(
            identities.insert((directory.name, directory.markers)),
            "duplicate traversal directory convention"
        );
    }
    directories
});

pub fn traversal_directories() -> &'static [&'static TraversalDirectory] {
    REGISTERED.as_slice()
}
