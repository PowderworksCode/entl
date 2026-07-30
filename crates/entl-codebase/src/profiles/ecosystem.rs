use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::{EcosystemId, LanguageProfile, language_profiles};

use super::registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EcosystemRole {
    PackageManager,
    BuildSystem,
    Runtime,
    Toolchain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestSelection {
    Default,
    Lockfile,
}

#[derive(Debug, Clone, Copy)]
pub struct EcosystemProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    pub roles: &'static [EcosystemRole],
    pub implied_languages: &'static [&'static LanguageProfile],
    pub manifest: Option<&'static str>,
    pub lockfiles: &'static [&'static str],
    pub selector_files: &'static [&'static str],
    pub gitignore_patterns: &'static [&'static str],
    pub manifest_selection: ManifestSelection,
}

impl EcosystemProfile {
    pub fn implies_language(&self, language: &LanguageProfile) -> bool {
        self.implied_languages
            .iter()
            .any(|candidate| std::ptr::eq(*candidate, language))
    }

    pub fn has_role(&self, role: EcosystemRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn lockfile_present(&self, directory: &Path) -> bool {
        self.lockfiles
            .iter()
            .any(|lockfile| directory.join(lockfile).is_file())
    }

    pub fn lockfile_description(&self) -> String {
        self.lockfiles.join(" or ")
    }
}

impl From<&EcosystemProfile> for EcosystemId {
    fn from(profile: &EcosystemProfile) -> Self {
        Self::new(profile.id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EcosystemRegistration(pub &'static EcosystemProfile);

registry::collect!(EcosystemRegistration);

static REGISTERED: LazyLock<Vec<&'static EcosystemProfile>> = LazyLock::new(|| {
    let mut profiles = registry::iter::<EcosystemRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.id);
    for pair in profiles.windows(2) {
        assert_ne!(pair[0].id, pair[1].id, "duplicate ecosystem profile ID");
    }
    for profile in &profiles {
        for language in profile.implied_languages {
            assert!(
                language_profiles()
                    .iter()
                    .any(|registered| std::ptr::eq(*registered, *language)),
                "ecosystem profile {:?} implies unregistered language {:?}",
                profile.id,
                language.id
            );
        }
    }
    let manifests = profiles
        .iter()
        .filter_map(|profile| profile.manifest)
        .collect::<BTreeSet<_>>();
    for manifest in manifests {
        assert_eq!(
            profiles
                .iter()
                .filter(|profile| {
                    profile.manifest == Some(manifest)
                        && matches!(profile.manifest_selection, ManifestSelection::Default)
                })
                .count(),
            1,
            "manifest {manifest:?} needs exactly one default ecosystem"
        );
    }
    profiles
});

pub fn ecosystem_profiles() -> &'static [&'static EcosystemProfile] {
    REGISTERED.as_slice()
}

pub fn ecosystem_profile(id: &str) -> Option<&'static EcosystemProfile> {
    ecosystem_profiles()
        .binary_search_by_key(&id, |profile| profile.id)
        .ok()
        .map(|index| ecosystem_profiles()[index])
}
