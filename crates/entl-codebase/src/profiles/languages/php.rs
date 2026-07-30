use super::{simple_language, syntax};

simple_language! {
    id: "php", name: "PHP", role: Programming,
    extensions: ["php"], filenames: [], shebangs: ["php"], comments: Some(&syntax::PHP),
    facets: [crate::STRUCTURED_CODE]
}
