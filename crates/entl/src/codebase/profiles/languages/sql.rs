use super::{simple_language, syntax};

simple_language! {
    id: "sql", name: "SQL", role: Programming,
    extensions: ["sql"], filenames: [], shebangs: [], comments: Some(&syntax::SQL)
}
