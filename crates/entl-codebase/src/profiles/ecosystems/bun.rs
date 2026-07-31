use crate::{
    EcosystemProfile, EcosystemRegistration, EcosystemRole, ManifestSelection,
    profiles::languages::javascript,
};

pub static PROFILE: EcosystemProfile = EcosystemProfile {
    id: "bun",
    display_name: "Bun",
    roles: &[EcosystemRole::PackageManager, EcosystemRole::Runtime],
    implied_languages: &[&javascript::PROFILE],
    manifest: Some("package.json"),
    lockfiles: &["bun.lock", "bun.lockb"],
    selector_files: &[],
    gitignore_patterns: &["node_modules/"],
    manifest_selection: ManifestSelection::Lockfile,
    dependency_pins: Some(super::npm::DEPENDENCY_PINS),
};

crate::profiles::registry::submit! {
    EcosystemRegistration(&PROFILE)
}
