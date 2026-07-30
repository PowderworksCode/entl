use crate::{
    LanguageConventions, LanguageProfile, LanguageRegistration, LanguageRole, STRUCTURED_CODE,
    TestLayoutDefaults,
};

use super::syntax;

pub static PROFILE: LanguageProfile = LanguageProfile {
    id: "rust",
    display_name: "Rust",
    extensions: &["rs"],
    source_extensions: &["rs"],
    filenames: &[],
    shebangs: &[],
    role: LanguageRole::Programming,
    facets: &[&STRUCTURED_CODE],
    comments: Some(&syntax::RUST),
    conventions: Some(LanguageConventions {
        typecheck: None,
        test_layout: TestLayoutDefaults {
            source_roots: &["src"],
            test_root: "tests",
            test_suffixes: &["", ".test", ".spec"],
        },
        inline_test_detector: inline_test,
    }),
    config_files: &[],
    package_dependencies: &[],
    supersedes: &[],
};

crate::profiles::registry::submit! {
    LanguageRegistration(&PROFILE)
}

fn inline_test(content: &str) -> Option<&'static str> {
    content.lines().find_map(|line| {
        let line = line.trim_start();
        if line.starts_with("#[cfg(test)]") {
            Some("#[cfg(test)]")
        } else if line.starts_with("#[test]") {
            Some("#[test]")
        } else if line.starts_with("mod tests") {
            Some("mod tests")
        } else {
            None
        }
    })
}
