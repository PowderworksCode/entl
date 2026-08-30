use super::simple_language;

simple_language! {
    id: "markdown", name: "Markdown", role: Documentation,
    extensions: ["md", "mdx", "markdown"], filenames: [], shebangs: [], comments: None
}
