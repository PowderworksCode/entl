use super::{simple_language, syntax};

simple_language! {
    id: "svelte", name: "Svelte", role: Programming,
    extensions: ["svelte"], filenames: [], shebangs: [], comments: Some(&syntax::SFC),
    facets: [crate::STRUCTURED_CODE, crate::STYLE_HOST, crate::COMPONENT_HOST]
}
