use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use entl_codebase::{
    BUN_ECOSYSTEM, CARGO_ECOSYSTEM, CodebaseInventory, EcosystemProfile, NPM_ECOSYSTEM,
    PNPM_ECOSYSTEM, YARN_ECOSYSTEM,
};
use serde_yaml_ng::Value;

use crate::{DependabotConfiguration, DependabotInventory, DependabotUpdate, Diagnostic};

pub use registry_inventory as registry;

#[derive(Debug, Clone, Copy)]
pub struct DependabotEcosystemProfile {
    pub ecosystem: &'static EcosystemProfile,
    pub package_ecosystem: &'static str,
    pub alternatives: &'static [&'static str],
}

impl DependabotEcosystemProfile {
    pub fn accepts(self, package_ecosystem: &str) -> bool {
        self.package_ecosystem == package_ecosystem
            || self.alternatives.contains(&package_ecosystem)
    }
}

pub struct DependabotEcosystemRegistration(pub &'static DependabotEcosystemProfile);

registry::collect!(DependabotEcosystemRegistration);

macro_rules! register {
    ($name:ident, $ecosystem:expr, $package_ecosystem:literal, $alternatives:expr) => {
        static $name: DependabotEcosystemProfile = DependabotEcosystemProfile {
            ecosystem: $ecosystem,
            package_ecosystem: $package_ecosystem,
            alternatives: $alternatives,
        };
        registry::submit! { DependabotEcosystemRegistration(&$name) }
    };
}

register!(CARGO, &CARGO_ECOSYSTEM, "cargo", &[]);
register!(BUN, &BUN_ECOSYSTEM, "bun", &["npm"]);
register!(NPM, &NPM_ECOSYSTEM, "npm", &[]);
register!(PNPM, &PNPM_ECOSYSTEM, "npm", &[]);
register!(YARN, &YARN_ECOSYSTEM, "npm", &[]);

static REGISTERED: LazyLock<Vec<&'static DependabotEcosystemProfile>> = LazyLock::new(|| {
    let mut profiles = registry::iter::<DependabotEcosystemRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.ecosystem.id);
    for pair in profiles.windows(2) {
        assert_ne!(
            pair[0].ecosystem.id, pair[1].ecosystem.id,
            "duplicate Dependabot ecosystem profile"
        );
    }
    profiles
});

pub fn dependabot_ecosystem_profiles() -> &'static [&'static DependabotEcosystemProfile] {
    REGISTERED.as_slice()
}

pub fn dependabot_ecosystem_profile(
    ecosystem: &EcosystemProfile,
) -> Option<&'static DependabotEcosystemProfile> {
    dependabot_ecosystem_profiles()
        .iter()
        .copied()
        .find(|profile| std::ptr::eq(profile.ecosystem, ecosystem))
}

pub(crate) fn inspect(codebase: &CodebaseInventory) -> DependabotInventory {
    let files = [
        PathBuf::from(".github/dependabot.yml"),
        PathBuf::from(".github/dependabot.yaml"),
    ]
    .into_iter()
    .filter(|path| codebase.has_file(path))
    .collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();
    if files.len() > 1 {
        diagnostics.push(Diagnostic {
            path: PathBuf::from(".github"),
            message: "both dependabot.yml and dependabot.yaml are present".to_owned(),
        });
    }
    let configuration = files.iter().next().and_then(|path| {
        parse(codebase, path) // straitjacket-allow:error-discard — the cause is recorded as a diagnostic just above
            .map_err(|message| {
                diagnostics.push(Diagnostic {
                    path: path.clone(),
                    message,
                });
            })
            .ok()
    });
    DependabotInventory {
        files,
        configuration,
        diagnostics,
    }
}

fn parse(codebase: &CodebaseInventory, path: &Path) -> Result<DependabotConfiguration, String> {
    let text = codebase
        .read_text(path)
        .map_err(|error| format!("Dependabot configuration is unreadable: {error}"))?;
    let value = serde_yaml_ng::from_str::<Value>(&text)
        .map_err(|error| format!("Dependabot configuration is invalid YAML: {error}"))?;
    if mapping_value(&value, "version").and_then(Value::as_u64) != Some(2) {
        return Err("Dependabot configuration must declare version: 2".to_owned());
    }
    let entries = mapping_value(&value, "updates")
        .and_then(Value::as_sequence)
        .ok_or_else(|| "Dependabot configuration must contain an updates list".to_owned())?;
    let mut updates = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let number = index + 1;
        if entry.as_mapping().is_none() {
            return Err(format!("Dependabot update {number} must be a mapping"));
        }
        let package_ecosystem = mapping_value(entry, "package-ecosystem")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("Dependabot update {number} needs a package-ecosystem"))?;
        let directory = mapping_value(entry, "directory");
        let directories = mapping_value(entry, "directories");
        if directory.is_some() && directories.is_some() {
            return Err(format!(
                "Dependabot update {number} cannot set both directory and directories"
            ));
        }
        let mut parsed_directories = Vec::new();
        if let Some(directory) = directory {
            parsed_directories.push(
                directory
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        format!("Dependabot update {number} directory must be a string")
                    })?
                    .to_owned(),
            );
        } else if let Some(directories) = directories {
            let values = directories
                .as_sequence()
                .ok_or_else(|| format!("Dependabot update {number} directories must be a list"))?;
            for value in values {
                parsed_directories.push(
                    value
                        .as_str()
                        .filter(|value| !value.trim().is_empty())
                        .ok_or_else(|| {
                            format!("Dependabot update {number} directories must contain strings")
                        })?
                        .to_owned(),
                );
            }
        }
        if parsed_directories.is_empty() {
            return Err(format!(
                "Dependabot update {number} needs directory or directories"
            ));
        }
        if mapping_value(entry, "schedule")
            .and_then(|schedule| mapping_value(schedule, "interval"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .is_none()
        {
            return Err(format!(
                "Dependabot update {number} needs schedule.interval"
            ));
        }
        parsed_directories.sort();
        parsed_directories.dedup();
        updates.push(DependabotUpdate {
            package_ecosystem: package_ecosystem.to_owned(),
            directories: parsed_directories,
        });
    }
    updates.sort();
    Ok(DependabotConfiguration {
        path: path.to_path_buf(),
        updates,
    })
}

fn mapping_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_mapping()?.get(Value::String(key.to_owned()))
}
