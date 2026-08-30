use super::{simple_language, syntax};

simple_language! {
    id: "yaml", name: "YAML", role: Data,
    extensions: ["yaml", "yml"], filenames: [], shebangs: [], comments: Some(&syntax::HASH)
}
