// The mirrored tests for `src/codebase/`, one file per source module.
//
// A test target resolves `mod` against its own directory rather than a
// subdirectory named after it, so each module states its path, and cargo
// builds only top-level files under tests/ as targets.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "codebase/support.rs"]
mod support;

#[path = "codebase/compiler.rs"]
mod compiler;
#[path = "codebase/discovery/cargo.rs"]
mod discovery_cargo;
#[path = "codebase/discovery/mod.rs"]
mod discovery_mod;
#[path = "codebase/discovery/node.rs"]
mod discovery_node;
#[path = "codebase/model/artifact.rs"]
mod model_artifact;
#[path = "codebase/model/file.rs"]
mod model_file;
#[path = "codebase/profiles/ecosystems/bun.rs"]
mod profiles_ecosystems_bun;
#[path = "codebase/profiles/ecosystems/cargo.rs"]
mod profiles_ecosystems_cargo;
#[path = "codebase/profiles/ecosystems/npm.rs"]
mod profiles_ecosystems_npm;
#[path = "codebase/profiles/ecosystems/pnpm.rs"]
mod profiles_ecosystems_pnpm;
#[path = "codebase/profiles/ecosystems/yarn.rs"]
mod profiles_ecosystems_yarn;
#[path = "codebase/profiles/languages/c.rs"]
mod profiles_languages_c;
#[path = "codebase/profiles/languages/cpp.rs"]
mod profiles_languages_cpp;
#[path = "codebase/profiles/languages/csharp.rs"]
mod profiles_languages_csharp;
#[path = "codebase/profiles/languages/css.rs"]
mod profiles_languages_css;
#[path = "codebase/profiles/languages/dockerfile.rs"]
mod profiles_languages_dockerfile;
#[path = "codebase/profiles/languages/go.rs"]
mod profiles_languages_go;
#[path = "codebase/profiles/languages/html.rs"]
mod profiles_languages_html;
#[path = "codebase/profiles/languages/java.rs"]
mod profiles_languages_java;
#[path = "codebase/profiles/languages/javascript.rs"]
mod profiles_languages_javascript;
#[path = "codebase/profiles/languages/json.rs"]
mod profiles_languages_json;
#[path = "codebase/profiles/languages/kotlin.rs"]
mod profiles_languages_kotlin;
#[path = "codebase/profiles/languages/less.rs"]
mod profiles_languages_less;
#[path = "codebase/profiles/languages/make.rs"]
mod profiles_languages_make;
#[path = "codebase/profiles/languages/markdown.rs"]
mod profiles_languages_markdown;
#[path = "codebase/profiles/languages/php.rs"]
mod profiles_languages_php;
#[path = "codebase/profiles/languages/python.rs"]
mod profiles_languages_python;
#[path = "codebase/profiles/languages/ruby.rs"]
mod profiles_languages_ruby;
#[path = "codebase/profiles/languages/rust.rs"]
mod profiles_languages_rust;
#[path = "codebase/profiles/languages/scala.rs"]
mod profiles_languages_scala;
#[path = "codebase/profiles/languages/scss.rs"]
mod profiles_languages_scss;
#[path = "codebase/profiles/languages/shell.rs"]
mod profiles_languages_shell;
#[path = "codebase/profiles/languages/sql.rs"]
mod profiles_languages_sql;
#[path = "codebase/profiles/languages/svelte.rs"]
mod profiles_languages_svelte;
#[path = "codebase/profiles/languages/swift.rs"]
mod profiles_languages_swift;
#[path = "codebase/profiles/languages/toml.rs"]
mod profiles_languages_toml;
#[path = "codebase/profiles/languages/typescript.rs"]
mod profiles_languages_typescript;
#[path = "codebase/profiles/languages/vue.rs"]
mod profiles_languages_vue;
#[path = "codebase/profiles/languages/yaml.rs"]
mod profiles_languages_yaml;
#[path = "codebase/profiles/languages/zig.rs"]
mod profiles_languages_zig;
#[path = "codebase/profiles/tools/documentation.rs"]
mod profiles_tools_documentation;
#[path = "codebase/profiles/tools/javascript.rs"]
mod profiles_tools_javascript;
#[path = "codebase/profiles/tools/rust.rs"]
mod profiles_tools_rust;
#[path = "codebase/profiles/tools/stylesheet.rs"]
mod profiles_tools_stylesheet;
#[path = "codebase/profiles/tools/system.rs"]
mod profiles_tools_system;
#[path = "codebase/profiles/tools/tauri.rs"]
mod profiles_tools_tauri;
#[path = "codebase/walk.rs"]
mod walk;
