use crate::{
    EcosystemProfile, EcosystemRegistration, EcosystemRole, ManifestSelection,
    profiles::languages::javascript,
};

pub static PROFILE: EcosystemProfile = EcosystemProfile {
    id: "pnpm",
    display_name: "pnpm",
    roles: &[EcosystemRole::PackageManager],
    implied_languages: &[&javascript::PROFILE],
    manifest: Some("package.json"),
    lockfiles: &["pnpm-lock.yaml"],
    selector_files: &["pnpm-workspace.yaml"],
    gitignore_patterns: &["node_modules/"],
    manifest_selection: ManifestSelection::Lockfile,
};

crate::profiles::registry::submit! {
    EcosystemRegistration(&PROFILE)
}
