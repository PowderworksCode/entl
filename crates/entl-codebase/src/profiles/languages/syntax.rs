use crate::CommentSyntax;

pub(super) static JS: CommentSyntax = CommentSyntax {
    line: &["//"],
    block: &[("/*", "*/")],
    documentation: &["/**"],
    quotes: &['"', '\''],
    multi_quotes: &["`"],
};

pub(super) static C_LIKE: CommentSyntax = CommentSyntax {
    line: &["//"],
    block: &[("/*", "*/")],
    documentation: &["///", "//!", "/**", "/*!"],
    quotes: &['"', '\''],
    multi_quotes: &[],
};

pub(super) static RUST: CommentSyntax = CommentSyntax {
    line: &["//"],
    block: &[("/*", "*/")],
    documentation: &["///", "//!", "/**", "/*!"],
    quotes: &['"'],
    multi_quotes: &[],
};

pub(super) static PHP: CommentSyntax = CommentSyntax {
    line: &["//", "#"],
    block: &[("/*", "*/")],
    documentation: &["/**"],
    quotes: &['"', '\''],
    multi_quotes: &[],
};

pub(super) static CSS: CommentSyntax = CommentSyntax {
    line: &[],
    block: &[("/*", "*/")],
    documentation: &[],
    quotes: &['"', '\''],
    multi_quotes: &[],
};

pub(super) static CSS_NESTED: CommentSyntax = CommentSyntax {
    line: &["//"],
    block: &[("/*", "*/")],
    documentation: &[],
    quotes: &['"', '\''],
    multi_quotes: &[],
};

pub(super) static HASH: CommentSyntax = CommentSyntax {
    line: &["#"],
    block: &[],
    documentation: &[],
    quotes: &['"', '\''],
    multi_quotes: &[],
};

pub(super) static PYTHON: CommentSyntax = CommentSyntax {
    line: &["#"],
    block: &[],
    documentation: &[],
    quotes: &['"', '\''],
    multi_quotes: &["\"\"\"", "'''"],
};

pub(super) static SQL: CommentSyntax = CommentSyntax {
    line: &["--"],
    block: &[("/*", "*/")],
    documentation: &[],
    quotes: &['"', '\''],
    multi_quotes: &[],
};

pub(super) static HTML: CommentSyntax = CommentSyntax {
    line: &[],
    block: &[("<!--", "-->")],
    documentation: &[],
    quotes: &[],
    multi_quotes: &[],
};

pub(super) static SFC: CommentSyntax = CommentSyntax {
    line: &["//"],
    block: &[("/*", "*/"), ("<!--", "-->")],
    documentation: &["/**"],
    quotes: &['"', '\''],
    multi_quotes: &["`"],
};
