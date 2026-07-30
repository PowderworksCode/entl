use super::{simple_language, syntax};

simple_language! {
    id: "make", name: "Make", role: Build,
    extensions: ["mk"], filenames: ["Makefile", "GNUmakefile"], shebangs: [],
    comments: Some(&syntax::HASH)
}
