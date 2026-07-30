use super::{simple_language, syntax};

simple_language! {
    id: "java", name: "Java", role: Programming,
    extensions: ["java"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
