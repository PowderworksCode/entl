// The mirrored tests for src/codebase/, one file per source module.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "codebase/support.rs"]
mod support;

#[path = "codebase/compiler.rs"]
mod compiler;
#[path = "codebase/discovery/cargo.rs"]
mod discovery_cargo;
#[path = "codebase/discovery/mod.rs"]
mod discovery_module;
#[path = "codebase/discovery/node.rs"]
mod discovery_node;
#[path = "codebase/error.rs"]
mod error;
#[path = "codebase/model/artifact.rs"]
mod model_artifact;
#[path = "codebase/model/codebase.rs"]
mod model_codebase;
#[path = "codebase/model/diagnostic.rs"]
mod model_diagnostic;
#[path = "codebase/model/file.rs"]
mod model_file;
#[path = "codebase/model/id.rs"]
mod model_id;
#[path = "codebase/model/mod.rs"]
mod model_module;
#[path = "codebase/model/package.rs"]
mod model_package;
#[path = "codebase/model/project.rs"]
mod model_project;
#[path = "codebase/model/workspace.rs"]
mod model_workspace;
#[path = "codebase/mod.rs"]
mod module;
#[path = "codebase/profiles/artifact.rs"]
mod profiles_artifact;
#[path = "codebase/profiles/artifacts.rs"]
mod profiles_artifacts;
#[path = "codebase/profiles/convention.rs"]
mod profiles_convention;
#[path = "codebase/profiles/ecosystem.rs"]
mod profiles_ecosystem;
#[path = "codebase/profiles/ecosystems/bun.rs"]
mod profiles_ecosystems_bun;
#[path = "codebase/profiles/ecosystems/cargo.rs"]
mod profiles_ecosystems_cargo;
#[path = "codebase/profiles/ecosystems/mod.rs"]
mod profiles_ecosystems_module;
#[path = "codebase/profiles/ecosystems/npm.rs"]
mod profiles_ecosystems_npm;
#[path = "codebase/profiles/ecosystems/pnpm.rs"]
mod profiles_ecosystems_pnpm;
#[path = "codebase/profiles/ecosystems/yarn.rs"]
mod profiles_ecosystems_yarn;
#[path = "codebase/profiles/facet.rs"]
mod profiles_facet;
#[path = "codebase/profiles/facets.rs"]
mod profiles_facets;
#[path = "codebase/profiles/language.rs"]
mod profiles_language;
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
#[path = "codebase/profiles/languages/mod.rs"]
mod profiles_languages_module;
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
#[path = "codebase/profiles/languages/syntax.rs"]
mod profiles_languages_syntax;
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
#[path = "codebase/profiles/mod.rs"]
mod profiles_module;
#[path = "codebase/profiles/tool.rs"]
mod profiles_tool;
#[path = "codebase/profiles/tools/documentation.rs"]
mod profiles_tools_documentation;
#[path = "codebase/profiles/tools/javascript.rs"]
mod profiles_tools_javascript;
#[path = "codebase/profiles/tools/mod.rs"]
mod profiles_tools_module;
#[path = "codebase/profiles/tools/rust.rs"]
mod profiles_tools_rust;
#[path = "codebase/profiles/tools/stylesheet.rs"]
mod profiles_tools_stylesheet;
#[path = "codebase/profiles/tools/system.rs"]
mod profiles_tools_system;
#[path = "codebase/profiles/tools/tauri.rs"]
mod profiles_tools_tauri;
#[path = "codebase/profiles/traversal.rs"]
mod profiles_traversal;
#[path = "codebase/profiles/verbosity.rs"]
mod profiles_verbosity;
#[path = "codebase/walk.rs"]
mod walk;
