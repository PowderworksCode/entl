mod c;
mod cpp;
mod csharp;
pub(crate) mod css;
mod dockerfile;
mod go;
mod html;
mod java;
pub(crate) mod javascript;
mod json;
mod kotlin;
pub(crate) mod less;
mod make;
mod markdown;
mod php;
mod python;
mod ruby;
pub(crate) mod rust;
mod scala;
pub(crate) mod scss;
pub(crate) mod shell;
mod sql;
mod svelte;
mod swift;
mod syntax;
mod toml;
pub(crate) mod typescript;
mod vue;
mod yaml;
mod zig;

macro_rules! simple_language {
    (
        id: $id:literal,
        name: $name:literal,
        role: $role:ident,
        extensions: [$($extension:literal),* $(,)?],
        filenames: [$($filename:literal),* $(,)?],
        shebangs: [$($shebang:literal),* $(,)?],
        comments: $comments:expr
        $(, facets: [$($facet:path),* $(,)?])?
        $(,)?
    ) => {
        pub static PROFILE: crate::codebase::LanguageProfile = crate::codebase::LanguageProfile {
            id: $id,
            display_name: $name,
            extensions: &[$($extension),*],
            source_extensions: &[$($extension),*],
            filenames: &[$($filename),*],
            shebangs: &[$($shebang),*],
            role: crate::codebase::LanguageRole::$role,
            facets: &[$($(&$facet),*)?],
            comments: $comments,
            conventions: None,
            config_files: &[],
            package_dependencies: &[],
            supersedes: &[],
        };

        crate::codebase::profiles::registry::submit! {
            crate::codebase::LanguageRegistration(&PROFILE)
        }
    };
}

pub(crate) use simple_language;
