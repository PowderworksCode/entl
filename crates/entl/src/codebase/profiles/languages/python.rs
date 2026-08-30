use super::{simple_language, syntax};

simple_language! {
    id: "python", name: "Python", role: Programming,
    extensions: ["py", "pyi"], filenames: [], shebangs: ["python"],
    comments: Some(&syntax::PYTHON), facets: [crate::codebase::STRUCTURED_CODE]
}

static BYTECODE_CACHE: crate::codebase::TraversalDirectory = crate::codebase::TraversalDirectory {
    name: "__pycache__",
    markers: &[],
};

crate::codebase::profiles::registry::submit! {
    crate::codebase::TraversalDirectoryRegistration(&BYTECODE_CACHE)
}
