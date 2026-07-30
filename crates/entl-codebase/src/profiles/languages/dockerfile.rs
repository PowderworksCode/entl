use super::{simple_language, syntax};

simple_language! {
    id: "dockerfile", name: "Dockerfile", role: Build,
    extensions: [], filenames: ["Dockerfile", "Containerfile"], shebangs: [],
    comments: Some(&syntax::HASH)
}
