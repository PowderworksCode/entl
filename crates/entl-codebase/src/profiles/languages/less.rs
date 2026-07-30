use super::{simple_language, syntax};

simple_language! {
    id: "less", name: "Less", role: Stylesheet,
    extensions: ["less"], filenames: [], shebangs: [], comments: Some(&syntax::CSS_NESTED),
    facets: [crate::STYLE_HOST]
}
