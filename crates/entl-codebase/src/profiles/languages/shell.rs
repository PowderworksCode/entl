use super::{simple_language, syntax};

simple_language! {
    id: "shell", name: "Shell", role: Programming,
    extensions: ["sh", "bash", "zsh", "fish"], filenames: [],
    shebangs: ["sh", "bash", "zsh", "fish"], comments: Some(&syntax::HASH),
    facets: [crate::STRUCTURED_CODE]
}
