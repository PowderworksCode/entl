use crate::{
    ArgumentPattern, BINARY_ARTIFACT, JAVASCRIPT_LANGUAGE, LanguageProfile, NAPI_ARTIFACT,
    SITE_ARTIFACT, TYPESCRIPT_LANGUAGE,
};

use super::super::{
    CiWorkload, CommandPattern, TaskKind, TestRetryConfiguration, TestRetryProfile,
    TestRetrySignal, ToolProfile, ToolRegistration,
};

const SCRIPT_LANGUAGES: &[&LanguageProfile] = &[&JAVASCRIPT_LANGUAGE, &TYPESCRIPT_LANGUAGE];

const PLAYWRIGHT_RETRIES: TestRetryConfiguration = TestRetryConfiguration {
    paths: &[
        "playwright.config.ts",
        "playwright.config.js",
        "playwright.config.mts",
        "playwright.config.mjs",
        "playwright.config.cts",
        "playwright.config.cjs",
    ],
    signals: &[TestRetrySignal::JavascriptProperty("retries")],
};

const JEST_RETRIES: TestRetryConfiguration = TestRetryConfiguration {
    paths: &[
        "jest.config.ts",
        "jest.config.js",
        "jest.config.mts",
        "jest.config.mjs",
        "jest.config.cts",
        "jest.config.cjs",
    ],
    signals: &[
        TestRetrySignal::JavascriptProperty("retries"),
        TestRetrySignal::JavascriptCall("retryTimes"),
    ],
};

const VITEST_RETRIES: TestRetryConfiguration = TestRetryConfiguration {
    paths: &[
        "vitest.config.ts",
        "vitest.config.js",
        "vitest.config.mts",
        "vitest.config.mjs",
        "vitest.config.cts",
        "vitest.config.cjs",
    ],
    signals: &[TestRetrySignal::JavascriptProperty("retry")],
};

static PACKAGE_MANAGER: ToolProfile = ToolProfile {
    id: "javascript-package-manager",
    programs: &["bun", "npm", "pnpm", "yarn"],
    languages: SCRIPT_LANGUAGES,
    commands: &[
        CommandPattern::tasks(&["test"], &[TaskKind::Test]),
        CommandPattern::produces(
            &["build"],
            &[ArgumentPattern::Exact("--compile")],
            &[],
            &[&BINARY_ARTIFACT],
        ),
        CommandPattern::produces(
            &["build"],
            &[ArgumentPattern::Prefix("--compile=")],
            &[],
            &[&BINARY_ARTIFACT],
        ),
        CommandPattern::produces(
            &["build"],
            &[
                ArgumentPattern::Exact("--target"),
                ArgumentPattern::Exact("browser"),
            ],
            &[
                ArgumentPattern::Exact("--compile"),
                ArgumentPattern::Prefix("--compile="),
            ],
            &[&SITE_ARTIFACT],
        ),
        CommandPattern::produces(
            &["build"],
            &[ArgumentPattern::Prefix("--target=browser")],
            &[
                ArgumentPattern::Exact("--compile"),
                ArgumentPattern::Prefix("--compile="),
            ],
            &[&SITE_ARTIFACT],
        ),
    ],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static TEST_RUNNER: ToolProfile = ToolProfile {
    id: "javascript-test-runner",
    programs: &["vitest", "jest", "playwright"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Test])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: Some(TestRetryProfile {
        arguments: &["--retries"],
        configurations: &[PLAYWRIGHT_RETRIES, JEST_RETRIES, VITEST_RETRIES],
    }),
};

static LINTER: ToolProfile = ToolProfile {
    id: "javascript-linter",
    programs: &["eslint", "oxlint"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static BIOME: ToolProfile = ToolProfile {
    id: "biome",
    programs: &["biome"],
    languages: SCRIPT_LANGUAGES,
    commands: &[
        CommandPattern::tasks(&["check"], &[TaskKind::Lint, TaskKind::Format]),
        CommandPattern::tasks(&["ci"], &[TaskKind::Lint, TaskKind::Format]),
        CommandPattern::tasks(&["lint"], &[TaskKind::Lint]),
        CommandPattern::tasks(&["format"], &[TaskKind::Format]),
    ],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static FORMATTER: ToolProfile = ToolProfile {
    id: "javascript-formatter",
    programs: &["prettier", "dprint"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Format])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static TYPESCRIPT: ToolProfile = ToolProfile {
    id: "typescript",
    programs: &["tsc", "tsgo", "vue-tsc"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Typecheck])],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static ASTRO: ToolProfile = ToolProfile {
    id: "astro",
    programs: &["astro"],
    languages: SCRIPT_LANGUAGES,
    commands: &[
        CommandPattern::tasks(&["check"], &[TaskKind::Typecheck]),
        CommandPattern::produces(&["build"], &[], &[], &[&SITE_ARTIFACT]),
    ],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static SITE_BUILDER: ToolProfile = ToolProfile {
    id: "site-builder",
    programs: &["vite", "next", "gatsby"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::produces(
        &["build"],
        &[],
        &[],
        &[&SITE_ARTIFACT],
    )],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Light,
    test_retry: None,
};

static NAPI: ToolProfile = ToolProfile {
    id: "napi",
    programs: &["napi"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::produces(
        &["build"],
        &[],
        &[],
        &[&NAPI_ARTIFACT],
    )],
    configuration_files: &[],
    package_json_keys: &[],
    ci_workload: CiWorkload::Heavy,
    test_retry: None,
};

crate::profiles::registry::submit! { ToolRegistration(&PACKAGE_MANAGER) }
crate::profiles::registry::submit! { ToolRegistration(&TEST_RUNNER) }
crate::profiles::registry::submit! { ToolRegistration(&LINTER) }
crate::profiles::registry::submit! { ToolRegistration(&BIOME) }
crate::profiles::registry::submit! { ToolRegistration(&FORMATTER) }
crate::profiles::registry::submit! { ToolRegistration(&TYPESCRIPT) }
crate::profiles::registry::submit! { ToolRegistration(&ASTRO) }
crate::profiles::registry::submit! { ToolRegistration(&SITE_BUILDER) }
crate::profiles::registry::submit! { ToolRegistration(&NAPI) }
