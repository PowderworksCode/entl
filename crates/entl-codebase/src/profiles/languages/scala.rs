use super::{simple_language, syntax};

simple_language! {
    id: "scala", name: "Scala", role: Programming,
    extensions: ["scala"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
