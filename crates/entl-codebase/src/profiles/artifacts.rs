use super::{ArtifactProfile, ArtifactRegistration};

pub static BINARY_ARTIFACT: ArtifactProfile = ArtifactProfile {
    id: "binary",
    display_name: "compiled binary",
    project_facets: &[],
    package_dependencies: &[],
    package_script_signals: &["bun build --compile"],
};

pub static NAPI_ARTIFACT: ArtifactProfile = ArtifactProfile {
    id: "napi",
    display_name: "Node native addon (napi-rs)",
    project_facets: &[],
    package_dependencies: &["@napi-rs/cli"],
    package_script_signals: &["napi build"],
};

pub static SITE_ARTIFACT: ArtifactProfile = ArtifactProfile {
    id: "site",
    display_name: "web/site bundle",
    project_facets: &["static-site"],
    package_dependencies: &[],
    package_script_signals: &[],
};

pub static TAURI_ARTIFACT: ArtifactProfile = ArtifactProfile {
    id: "tauri",
    display_name: "Tauri desktop application",
    project_facets: &["tauri"],
    package_dependencies: &[],
    package_script_signals: &[],
};

crate::profiles::registry::submit! { ArtifactRegistration(&BINARY_ARTIFACT) }
crate::profiles::registry::submit! { ArtifactRegistration(&NAPI_ARTIFACT) }
crate::profiles::registry::submit! { ArtifactRegistration(&SITE_ARTIFACT) }
crate::profiles::registry::submit! { ArtifactRegistration(&TAURI_ARTIFACT) }
