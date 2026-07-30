use crate::{
    COMPONENT_HOST, LanguageConventions, LanguageProfile, LanguageRegistration, LanguageRole,
    STRUCTURED_CODE, STYLE_HOST, TestLayoutDefaults, TypecheckConvention,
};

use super::syntax;

pub static PROFILE: LanguageProfile = LanguageProfile {
    id: "javascript",
    display_name: "JavaScript",
    extensions: &["js", "jsx", "mjs", "cjs"],
    source_extensions: &["js", "jsx", "mjs", "cjs"],
    filenames: &[],
    shebangs: &["node", "bun", "deno"],
    role: LanguageRole::Programming,
    facets: &[&STRUCTURED_CODE, &STYLE_HOST, &COMPONENT_HOST],
    comments: Some(&syntax::JS),
    conventions: Some(LanguageConventions {
        typecheck: Some(TypecheckConvention {
            config_files: &["jsconfig.json", "tsconfig.json"],
        }),
        test_layout: TestLayoutDefaults {
            source_roots: &["src"],
            test_root: "tests",
            test_suffixes: &["", ".test", ".spec"],
        },
        inline_test_detector: inline_test,
    }),
    config_files: &["jsconfig.json"],
    package_dependencies: &[],
    supersedes: &[],
};

crate::profiles::registry::submit! {
    LanguageRegistration(&PROFILE)
}

pub(super) fn inline_test(content: &str) -> Option<&'static str> {
    content.lines().find_map(|line| {
        let line = line.trim_start();
        if line.starts_with("describe(") || line.starts_with("describe.") {
            Some("describe")
        } else if line.starts_with("it(") || line.starts_with("it.") {
            Some("it")
        } else if line.starts_with("test(") || line.starts_with("test.") {
            Some("test")
        } else if line.starts_with("import ")
            && (line.contains("from \"vitest\"")
                || line.contains("from 'vitest'")
                || line.contains("@jest/globals")
                || line.contains("node:test"))
        {
            Some("test framework import")
        } else {
            None
        }
    })
}
