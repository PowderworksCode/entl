use super::{simple_language, syntax};

simple_language! {
    id: "scss", name: "SCSS", role: Stylesheet,
    extensions: ["scss", "sass"], filenames: [], shebangs: [],
    comments: Some(&syntax::CSS_NESTED), facets: [crate::STYLE_HOST]
}
