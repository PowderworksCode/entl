use super::{simple_language, syntax};

simple_language! {
    id: "ruby", name: "Ruby", role: Programming,
    extensions: ["rb", "rake"], filenames: ["Gemfile", "Rakefile"], shebangs: ["ruby"],
    comments: Some(&syntax::HASH), facets: [crate::STRUCTURED_CODE]
}
