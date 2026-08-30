// Tests for `src/codebase/profiles/tools/javascript.rs`.
//
// A tool profile is how a command in a workflow becomes a typed fact. If
// the programs it claims are wrong the tool is simply never recognised,
// and nothing else in the system notices.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::tool_profile;

#[test]
fn javascript_package_manager_is_registered_with_its_programs() {
    let profile = tool_profile("javascript-package-manager")
        .expect("javascript-package-manager is registered");
    assert_eq!(profile.id, "javascript-package-manager");
    assert_eq!(profile.programs, ["bun", "npm", "pnpm", "yarn"]);
}

#[test]
fn javascript_test_runner_is_registered_with_its_programs() {
    let profile =
        tool_profile("javascript-test-runner").expect("javascript-test-runner is registered");
    assert_eq!(profile.id, "javascript-test-runner");
    assert_eq!(profile.programs, ["vitest", "jest", "playwright"]);
}

#[test]
fn javascript_linter_is_registered_with_its_programs() {
    let profile = tool_profile("javascript-linter").expect("javascript-linter is registered");
    assert_eq!(profile.id, "javascript-linter");
    assert_eq!(profile.programs, ["eslint", "oxlint"]);
}

#[test]
fn biome_is_registered_with_its_programs() {
    let profile = tool_profile("biome").expect("biome is registered");
    assert_eq!(profile.id, "biome");
    assert_eq!(profile.programs, ["biome"]);
}

#[test]
fn javascript_formatter_is_registered_with_its_programs() {
    let profile = tool_profile("javascript-formatter").expect("javascript-formatter is registered");
    assert_eq!(profile.id, "javascript-formatter");
    assert_eq!(profile.programs, ["prettier", "dprint"]);
}

#[test]
fn typescript_is_registered_with_its_programs() {
    let profile = tool_profile("typescript").expect("typescript is registered");
    assert_eq!(profile.id, "typescript");
    assert_eq!(profile.programs, ["tsc", "tsgo", "vue-tsc"]);
}

#[test]
fn astro_is_registered_with_its_programs() {
    let profile = tool_profile("astro").expect("astro is registered");
    assert_eq!(profile.id, "astro");
    assert_eq!(profile.programs, ["astro"]);
}

#[test]
fn site_builder_is_registered_with_its_programs() {
    let profile = tool_profile("site-builder").expect("site-builder is registered");
    assert_eq!(profile.id, "site-builder");
    assert_eq!(profile.programs, ["vite", "next", "gatsby"]);
}

#[test]
fn napi_is_registered_with_its_programs() {
    let profile = tool_profile("napi").expect("napi is registered");
    assert_eq!(profile.id, "napi");
    assert_eq!(profile.programs, ["napi"]);
}
