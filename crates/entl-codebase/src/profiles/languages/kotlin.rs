use super::{simple_language, syntax};

simple_language! {
    id: "kotlin", name: "Kotlin", role: Programming,
    extensions: ["kt", "kts"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
