use super::{simple_language, syntax};

simple_language! {
    id: "c-sharp", name: "C#", role: Programming,
    extensions: ["cs"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
