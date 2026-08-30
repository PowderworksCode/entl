use super::{simple_language, syntax};

simple_language! {
    id: "toml", name: "TOML", role: Data,
    extensions: ["toml"], filenames: [], shebangs: [], comments: Some(&syntax::HASH)
}
