use super::{simple_language, syntax};

simple_language! {
    id: "python", name: "Python", role: Programming,
    extensions: ["py", "pyi"], filenames: [], shebangs: ["python"],
    comments: Some(&syntax::PYTHON), facets: [crate::STRUCTURED_CODE]
}

static BYTECODE_CACHE: crate::TraversalDirectory = crate::TraversalDirectory {
    name: "__pycache__",
    markers: &[],
};

crate::profiles::registry::submit! {
    crate::TraversalDirectoryRegistration(&BYTECODE_CACHE)
}
