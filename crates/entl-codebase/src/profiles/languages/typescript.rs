use crate::{
    COMPONENT_HOST, LanguageConventions, LanguageProfile, LanguageRegistration, LanguageRole,
    STRUCTURED_CODE, STYLE_HOST, TestLayoutDefaults, TypecheckConvention,
};

use super::javascript::{PROFILE as JAVASCRIPT, inline_test};
use super::syntax;

pub static PROFILE: LanguageProfile = LanguageProfile {
    id: "typescript",
    display_name: "TypeScript",
    extensions: &["ts", "tsx", "mts", "cts"],
    source_extensions: &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"],
    filenames: &[],
    shebangs: &[],
    role: LanguageRole::Programming,
    facets: &[&STRUCTURED_CODE, &STYLE_HOST, &COMPONENT_HOST],
    comments: Some(&syntax::JS),
    conventions: Some(LanguageConventions {
        typecheck: Some(TypecheckConvention {
            config_files: &["tsconfig.json"],
        }),
        test_layout: TestLayoutDefaults {
            source_roots: &["src"],
            test_root: "tests",
            test_suffixes: &["", ".test", ".spec"],
        },
        inline_test_detector: inline_test,
    }),
    config_files: &["tsconfig.json"],
    package_dependencies: &["typescript"],
    supersedes: &[&JAVASCRIPT],
};

crate::profiles::registry::submit! {
    LanguageRegistration(&PROFILE)
}
