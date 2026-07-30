use crate::{
    EcosystemProfile, EcosystemRegistration, EcosystemRole, ManifestSelection,
    profiles::languages::javascript,
};

pub static PROFILE: EcosystemProfile = EcosystemProfile {
    id: "npm",
    display_name: "npm",
    roles: &[EcosystemRole::PackageManager],
    implied_languages: &[&javascript::PROFILE],
    manifest: Some("package.json"),
    lockfiles: &["package-lock.json", "npm-shrinkwrap.json"],
    selector_files: &[],
    gitignore_patterns: &["node_modules/"],
    manifest_selection: ManifestSelection::Default,
};

crate::profiles::registry::submit! {
    EcosystemRegistration(&PROFILE)
}

macro_rules! traversal_directory {
    ($static_name:ident, $name:literal) => {
        static $static_name: crate::TraversalDirectory = crate::TraversalDirectory {
            name: $name,
            markers: &["package.json"],
        };
        crate::profiles::registry::submit! {
            crate::TraversalDirectoryRegistration(&$static_name)
        }
    };
}

static NODE_MODULES: crate::TraversalDirectory = crate::TraversalDirectory {
    name: "node_modules",
    markers: &[],
};

crate::profiles::registry::submit! {
    crate::TraversalDirectoryRegistration(&NODE_MODULES)
}

traversal_directory!(DIST, "dist");
traversal_directory!(BUILD, "build");
traversal_directory!(NEXT, ".next");
traversal_directory!(TURBO, ".turbo");
traversal_directory!(COVERAGE, "coverage");
