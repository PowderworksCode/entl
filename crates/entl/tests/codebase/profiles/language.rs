// Tests for `src/codebase/profiles/language.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{
    COMPONENT_HOST, JAVASCRIPT_LANGUAGE, RUST_LANGUAGE, SHELL_LANGUAGE, STRUCTURED_CODE,
    STYLE_HOST, TYPESCRIPT_LANGUAGE, language_facet, language_facets, language_profile,
};
use std::path::Path;

#[test]
fn language_profiles_colocate_detection_and_syntax() {
    let rust = language_profile("rust").unwrap();
    assert!(std::ptr::eq(rust, &RUST_LANGUAGE));
    assert!(rust.detects_source(Path::new("src/lib.rs")));
    assert_eq!(rust.comments.unwrap().line, ["//"]);

    let typescript = language_profile("typescript").unwrap();
    let javascript = language_profile("javascript").unwrap();
    let shell = language_profile("shell").unwrap();
    assert!(std::ptr::eq(typescript, &TYPESCRIPT_LANGUAGE));
    assert!(std::ptr::eq(javascript, &JAVASCRIPT_LANGUAGE));
    assert!(std::ptr::eq(shell, &SHELL_LANGUAGE));
    assert!(typescript.supersedes(javascript));
    assert!(typescript.accepts_source(Path::new("src/legacy.js")));
    assert!(typescript.has_facet(&STRUCTURED_CODE));
    assert!(typescript.has_facet(&STYLE_HOST));
    assert!(typescript.has_facet(&COMPONENT_HOST));
    assert!(rust.has_facet(&STRUCTURED_CODE));
    assert!(!rust.has_facet(&STYLE_HOST));
    assert!(std::ptr::eq(
        language_facet("style-host").unwrap(),
        &STYLE_HOST
    ));
    assert_eq!(
        language_facets()
            .iter()
            .map(|facet| facet.id)
            .collect::<Vec<_>>(),
        ["component-host", "structured-code", "style-host"]
    );
}
