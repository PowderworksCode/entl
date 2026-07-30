use super::{simple_language, syntax};

simple_language! {
    id: "c", name: "C", role: Programming,
    extensions: ["c", "h"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
