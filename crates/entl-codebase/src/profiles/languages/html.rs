use super::{simple_language, syntax};

simple_language! {
    id: "html", name: "HTML", role: Markup,
    extensions: ["html", "htm"], filenames: [], shebangs: [], comments: Some(&syntax::HTML),
    facets: [crate::STYLE_HOST]
}
