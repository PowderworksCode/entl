use super::{simple_language, syntax};

simple_language! {
    id: "css", name: "CSS", role: Stylesheet,
    extensions: ["css"], filenames: [], shebangs: [], comments: Some(&syntax::CSS),
    facets: [crate::codebase::STYLE_HOST]
}
