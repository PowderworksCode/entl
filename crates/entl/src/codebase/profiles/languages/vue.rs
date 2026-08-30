use super::{simple_language, syntax};

simple_language! {
    id: "vue", name: "Vue", role: Programming,
    extensions: ["vue"], filenames: [], shebangs: [], comments: Some(&syntax::SFC),
    facets: [crate::codebase::STRUCTURED_CODE, crate::codebase::STYLE_HOST, crate::codebase::COMPONENT_HOST]
}
