//! Making source readable by a grammar that predates it.
//!
//! A language moves faster than its grammar. Rust's standard library is written
//! against nightly and uses syntax no released `tree-sitter-rust` accepts —
//! `const trait`, `impl const`, `[const]` bounds — and tree-sitter rejects the
//! *whole file* when any part of it is beyond what it knows. One unsupported
//! keyword therefore removes everything beside it, which is how a third of
//! `core` came to contribute nothing at all.
//!
//! These rewrites blank the offending keyword and nothing else. They are a
//! stopgap, kept honest by two rules:
//!
//! - **Byte length never changes.** Every span a consumer produces is an offset
//!   into the source, so a rewrite that shifted anything would silently move
//!   every reported location. Keywords are replaced by spaces of equal width.
//! - **They only run on source that already failed.** A file the grammar
//!   accepts is never touched, so the risk is confined to files that would
//!   otherwise be dropped entirely.
//!
//! Each rewrite is a language moving ahead of its grammar, so each should be
//! removed when the grammar catches up.

/// A keyword to blank out, and where it has to appear to be blanked.
struct Rewrite {
    /// The text to remove.
    keyword: &'static str,
    /// Text that must immediately precede it, ignoring spaces.
    after: Option<&'static str>,
    /// Text that must immediately follow it, ignoring spaces.
    before: Option<&'static str>,
    /// Why the grammar cannot read it.
    reason: &'static str,
}

/// Syntax released grammars do not accept.
const RUST_REWRITES: &[Rewrite] = &[
    Rewrite {
        keyword: "const",
        before: Some("trait"),
        after: None,
        reason: "const traits are unstable, and `trait_item` admits only `unsafe`",
    },
    Rewrite {
        keyword: "const",
        after: Some("impl"),
        before: None,
        reason: "`impl const Trait for Ty` is unstable",
    },
    Rewrite {
        keyword: "[const]",
        after: None,
        before: None,
        reason: "conditionally const bounds are unstable",
    },
    Rewrite {
        keyword: "~const",
        after: None,
        before: None,
        reason: "the earlier spelling of conditionally const bounds",
    },
    Rewrite {
        keyword: "become",
        after: None,
        before: None,
        reason: "explicit tail calls are unstable",
    },
    Rewrite {
        keyword: "raw const",
        after: Some("&"),
        before: None,
        reason: "raw borrows are unstable; the borrow itself still parses",
    },
    Rewrite {
        keyword: "raw mut",
        after: Some("&"),
        before: None,
        reason: "raw borrows are unstable; the borrow itself still parses",
    },
    Rewrite {
        keyword: "yield",
        after: None,
        before: None,
        reason: "generators are unstable",
    },
    Rewrite {
        keyword: "try",
        after: None,
        before: Some("{"),
        reason: "try blocks are unstable; the block itself still parses",
    },
    Rewrite {
        keyword: "auto",
        after: None,
        before: Some("trait"),
        reason: "auto traits are unstable",
    },
];

/// What a rewrite pass changed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rewritten {
    pub source: Vec<u8>,
    /// The reasons that applied, for reporting rather than for silence.
    pub reasons: Vec<&'static str>,
}

/// Blank out syntax the grammar for `language` cannot read.
///
/// Returns `None` when nothing applied, so a caller can tell "no rewrite was
/// needed" from "a rewrite was made".
pub fn neutralize(language: impl AsRef<str>, source: &[u8]) -> Option<Rewritten> {
    let language = language.as_ref();
    if language != "rust" {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?;
    let mut output = text.to_owned();
    let mut reasons = Vec::new();
    for rewrite in RUST_REWRITES {
        if blank_all(&mut output, rewrite) {
            reasons.push(rewrite.reason);
        }
    }
    if reasons.is_empty() {
        return None;
    }
    debug_assert_eq!(
        output.len(),
        text.len(),
        "a rewrite must not move any byte, or every span shifts"
    );
    Some(Rewritten {
        source: output.into_bytes(),
        reasons,
    })
}

/// Replace every qualifying occurrence with spaces, in place.
fn blank_all(text: &mut String, rewrite: &Rewrite) -> bool {
    let mut blanked = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(rewrite.keyword) {
        let start = from + offset;
        let end = start + rewrite.keyword.len();
        from = end;
        if !is_word_boundary(text, start, end) || !context_matches(text, start, end, rewrite) {
            continue;
        }
        text.replace_range(start..end, &" ".repeat(rewrite.keyword.len()));
        blanked = true;
    }
    blanked
}

/// Whether the occurrence stands alone rather than sitting inside a longer word.
fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let joins = |character: Option<char>| {
        character.is_some_and(|character| character.is_alphanumeric() || character == '_')
    };
    !joins(before) && !joins(after)
}

fn context_matches(text: &str, start: usize, end: usize, rewrite: &Rewrite) -> bool {
    if let Some(after) = rewrite.after
        && !text[..start].trim_end().ends_with(after)
    {
        return false;
    }
    if let Some(before) = rewrite.before
        && !text[end..].trim_start().starts_with(before)
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite(source: &str) -> Option<String> {
        neutralize("rust", source.as_bytes())
            .map(|rewritten| String::from_utf8(rewritten.source).unwrap())
    }

    #[test]
    fn a_rewrite_never_moves_a_byte() {
        let source = "pub const trait Iterator { fn next(&mut self); }";
        let rewritten = rewrite(source).expect("const trait is rewritten");
        assert_eq!(rewritten.len(), source.len());
        assert!(
            rewritten.contains("pub       trait Iterator"),
            "{rewritten}"
        );
    }

    #[test]
    fn each_unsupported_spelling_is_covered() {
        assert!(
            rewrite("impl const Add for Foo {}")
                .unwrap()
                .contains("impl       Add")
        );
        assert!(
            !rewrite("fn f<T: [const] Add>() {}")
                .unwrap()
                .contains("[const]")
        );
        assert!(
            !rewrite("fn f<T: ~const Add>() {}")
                .unwrap()
                .contains("~const")
        );
        assert!(
            !rewrite("fn f() { become g(); }")
                .unwrap()
                .contains("become")
        );
    }

    #[test]
    fn later_additions_are_covered_too() {
        assert!(
            !rewrite("let p = &raw const x;")
                .unwrap()
                .contains("raw const")
        );
        assert!(!rewrite("let p = &raw mut x;").unwrap().contains("raw mut"));
        assert!(!rewrite("fn f() { yield 1; }").unwrap().contains("yield"));
        assert!(
            !rewrite("fn f() { let r = try { 1 }; }")
                .unwrap()
                .contains("try {")
        );
        assert!(
            !rewrite("pub auto trait Send {}")
                .unwrap()
                .contains("auto trait")
        );
    }

    #[test]
    fn ordinary_source_is_left_alone() {
        assert!(rewrite("pub const MAX: u32 = 1;").is_none());
        assert!(rewrite("pub trait Iterator {}").is_none());
        assert!(rewrite("impl Add for Foo {}").is_none());
        // `try` and `auto` only matter in the positions the grammar cannot read
        assert!(rewrite("fn try_parse() {}").is_none());
        assert!(rewrite("let auto = 1;").is_none());
    }

    #[test]
    fn a_word_that_merely_contains_a_keyword_is_untouched() {
        // `become` inside a longer identifier is not the keyword
        assert!(rewrite("fn becomes_ready() {}").is_none());
        assert!(rewrite("const CONSTANT: u32 = 1;").is_none());
    }

    #[test]
    fn a_rewrite_says_why_it_happened() {
        let rewritten = neutralize("rust", b"pub const trait T {}").unwrap();
        assert!(
            rewritten
                .reasons
                .iter()
                .any(|reason| reason.contains("const traits")),
            "{:?}",
            rewritten.reasons
        );
    }

    #[test]
    fn other_languages_are_not_rewritten() {
        assert!(neutralize("typescript", b"const trait = 1;").is_none());
    }
}
