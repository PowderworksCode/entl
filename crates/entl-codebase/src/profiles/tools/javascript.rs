use crate::{
    ArgumentPattern, BINARY_ARTIFACT, JAVASCRIPT_LANGUAGE, LanguageProfile, NAPI_ARTIFACT,
    SITE_ARTIFACT, TYPESCRIPT_LANGUAGE,
};

use super::super::{CommandPattern, TaskKind, ToolProfile, ToolRegistration};

const SCRIPT_LANGUAGES: &[&LanguageProfile] = &[&JAVASCRIPT_LANGUAGE, &TYPESCRIPT_LANGUAGE];

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
};

static TEST_RUNNER: ToolProfile = ToolProfile {
    id: "javascript-test-runner",
    programs: &["vitest", "jest", "playwright"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Test])],
};

static LINTER: ToolProfile = ToolProfile {
    id: "javascript-linter",
    programs: &["eslint", "oxlint"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Lint])],
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
};

static FORMATTER: ToolProfile = ToolProfile {
    id: "javascript-formatter",
    programs: &["prettier", "dprint"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Format])],
};

static TYPESCRIPT: ToolProfile = ToolProfile {
    id: "typescript",
    programs: &["tsc", "tsgo", "vue-tsc"],
    languages: SCRIPT_LANGUAGES,
    commands: &[CommandPattern::tasks(&[], &[TaskKind::Typecheck])],
};

static ASTRO: ToolProfile = ToolProfile {
    id: "astro",
    programs: &["astro"],
    languages: SCRIPT_LANGUAGES,
    commands: &[
        CommandPattern::tasks(&["check"], &[TaskKind::Typecheck]),
        CommandPattern::produces(&["build"], &[], &[], &[&SITE_ARTIFACT]),
    ],
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
