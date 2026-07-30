use super::{simple_language, syntax};

simple_language! {
    id: "vue", name: "Vue", role: Programming,
    extensions: ["vue"], filenames: [], shebangs: [], comments: Some(&syntax::SFC),
    facets: [crate::STRUCTURED_CODE, crate::STYLE_HOST, crate::COMPONENT_HOST]
}
