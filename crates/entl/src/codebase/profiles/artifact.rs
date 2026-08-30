use std::collections::BTreeSet;
use std::sync::LazyLock;

use super::registry;
use crate::codebase::ArtifactId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    pub project_facets: &'static [&'static str],
    pub package_dependencies: &'static [&'static str],
    pub package_script_signals: &'static [&'static str],
}

impl From<&ArtifactProfile> for ArtifactId {
    fn from(profile: &ArtifactProfile) -> Self {
        Self::from(profile.id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactRegistration(pub &'static ArtifactProfile);

registry::collect!(ArtifactRegistration);

static REGISTERED: LazyLock<Vec<&'static ArtifactProfile>> = LazyLock::new(|| {
    let mut profiles = registry::iter::<ArtifactRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.id);
    let mut ids = BTreeSet::new();
    for profile in &profiles {
        assert!(ids.insert(profile.id), "duplicate artifact profile ID");
    }
    profiles
});

pub fn artifact_profiles() -> &'static [&'static ArtifactProfile] {
    REGISTERED.as_slice()
}

pub fn artifact_profile(id: &str) -> Option<&'static ArtifactProfile> {
    artifact_profiles()
        .binary_search_by_key(&id, |profile| profile.id)
        .ok()
        .map(|index| artifact_profiles()[index])
}
