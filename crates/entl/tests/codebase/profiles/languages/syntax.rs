// Tests for `src/codebase/profiles/languages/syntax.rs`.
//
// The comment syntaxes are `pub(super)`, so they are reached the way anything
// else reaches them: through the language that claims one. What is worth
// pinning is that a language whose comments matter has them, because the line
// counter uses this to tell code from prose.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::language_profile;

#[test]
fn the_c_like_languages_share_a_comment_syntax() {
    for id in ["rust", "c", "cpp", "java"] {
        let profile = language_profile(id).expect("the language is registered");
        let comments = profile
            .comments
            .unwrap_or_else(|| panic!("{id} declares a comment syntax"));
        assert!(
            comments.line.contains(&"//"),
            "{id} uses // for line comments"
        );
    }
}

/// A language with no comment syntax says so rather than claiming an empty one:
/// the two mean different things to the counter.
#[test]
fn a_language_without_comments_declares_none() {
    let json = language_profile("json").expect("json is registered");
    assert!(json.comments.is_none());
}
