use crate::{RUST_LANGUAGE, SITE_ARTIFACT, TAURI_ARTIFACT, TYPESCRIPT_LANGUAGE};

use super::super::{CiWorkload, CommandPattern, ToolProfile, ToolRegistration};

static TAURI: ToolProfile = ToolProfile {
    id: "tauri",
    programs: &["tauri"],
    languages: &[&RUST_LANGUAGE, &TYPESCRIPT_LANGUAGE],
    commands: &[CommandPattern::produces(
        &["build"],
        &[],
        &[],
        &[&TAURI_ARTIFACT, &SITE_ARTIFACT],
    )],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Heavy,
    test_retry: None,
};

crate::profiles::registry::submit! { ToolRegistration(&TAURI) }
