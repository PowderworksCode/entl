use super::{simple_language, syntax};

simple_language! {
    id: "cpp", name: "C++", role: Programming,
    extensions: ["cc", "cpp", "cxx", "hh", "hpp", "hxx"], filenames: [], shebangs: [],
    comments: Some(&syntax::C_LIKE), facets: [crate::STRUCTURED_CODE]
}
