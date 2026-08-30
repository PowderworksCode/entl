use crate::codebase::{
    DependencyPinPolicy, DependencyPinSyntax, EcosystemProfile, EcosystemRegistration,
    EcosystemRole, ManifestSelection, profiles::languages::rust,
};

const DEPENDENCY_PINS: DependencyPinPolicy = DependencyPinPolicy {
    syntax: DependencyPinSyntax::CargoExactRequirement,
    advisory: true,
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
    dependency_pins: Some(DEPENDENCY_PINS),
};

crate::codebase::profiles::registry::submit! {
    EcosystemRegistration(&PROFILE)
}

static TARGET: crate::codebase::TraversalDirectory = crate::codebase::TraversalDirectory {
    name: "target",
    markers: &["Cargo.toml"],
};

crate::codebase::profiles::registry::submit! {
    crate::codebase::TraversalDirectoryRegistration(&TARGET)
}
