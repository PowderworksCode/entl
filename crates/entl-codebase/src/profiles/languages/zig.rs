use super::{simple_language, syntax};

simple_language! {
    id: "zig", name: "Zig", role: Programming,
    extensions: ["zig"], filenames: [], shebangs: [], comments: Some(&syntax::C_LIKE),
    facets: [crate::STRUCTURED_CODE]
}
