use super::{simple_language, syntax};

simple_language! {
    id: "svelte", name: "Svelte", role: Programming,
    extensions: ["svelte"], filenames: [], shebangs: [], comments: Some(&syntax::SFC),
    facets: [crate::codebase::STRUCTURED_CODE, crate::codebase::STYLE_HOST, crate::codebase::COMPONENT_HOST]
}
