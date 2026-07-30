use super::{simple_language, syntax};

simple_language! {
    id: "go", name: "Go", role: Programming,
    extensions: ["go"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
