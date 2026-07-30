use crate::{
    EcosystemProfile, EcosystemRegistration, EcosystemRole, ManifestSelection,
    profiles::languages::rust,
};

pub static PROFILE: EcosystemProfile = EcosystemProfile {
    id: "cargo",
    display_name: "Cargo",
    roles: &[EcosystemRole::PackageManager, EcosystemRole::BuildSystem],
    implied_languages: &[&rust::PROFILE],
    manifest: Some("Cargo.toml"),
    lockfiles: &["Cargo.lock"],
    selector_files: &[],
    gitignore_patterns: &["target/"],
    manifest_selection: ManifestSelection::Default,
};

crate::profiles::registry::submit! {
    EcosystemRegistration(&PROFILE)
}

static TARGET: crate::TraversalDirectory = crate::TraversalDirectory {
    name: "target",
    markers: &["Cargo.toml"],
};

crate::profiles::registry::submit! {
    crate::TraversalDirectoryRegistration(&TARGET)
}
