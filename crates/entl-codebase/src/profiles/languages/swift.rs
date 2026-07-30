use super::{simple_language, syntax};

simple_language! {
    id: "swift", name: "Swift", role: Programming,
    extensions: ["swift"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
