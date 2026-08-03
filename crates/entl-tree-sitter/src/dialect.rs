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
//!
//! # Two kinds of rewrite, and why the difference is recorded
//!
//! Blanking a keyword the grammar cannot read leaves what the code *says*
//! intact: `pub const trait Iterator` still declares `Iterator`, and every
//! consumer sees the same declaration it would have seen. Not every gap can be
//! closed that cheaply. Zig writes a type that depends on a comptime condition
//! as `if (cond) A else B`, and the only length-preserving way to make it
//! readable is to pick a branch — after which the signature is narrower than
//! the one in the file.
//!
//! That is still worth doing, because the alternative is losing every
//! declaration beside it, but it is not the same promise. [`Rewritten`] carries
//! `narrowed` so a consumer that reports source text — rather than only
//! locating it — can tell the two apart. Nothing here reports a rewritten
//! signature as faithful.
//!
//! Neither kind lexes, so a rewrite can fire inside a comment or a string
//! literal. It costs nothing when it does: the bytes stay put, and
//! [`crate::ParserRuntime::parse`] keeps a rewrite only when the retry parses
//! cleanly, so a change that did not help is discarded rather than kept.

/// What a rewrite costs in fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fidelity {
    /// The construct is removed and nothing the code says changes with it.
    Preserved,
    /// A choice the source left open is made. The bytes stay where they were
    /// and the file parses, but it now says something narrower than it did.
    Narrowed,
}

/// How a rewrite finds what the grammar cannot read, and what it does about it.
enum Rule {
    /// Replace a keyword with spaces where it stands alone and its neighbours
    /// match.
    Blank {
        /// The text to remove.
        keyword: &'static str,
        /// Text that must immediately precede it, ignoring spaces.
        after: Option<&'static str>,
        /// Text that must immediately follow it, ignoring spaces.
        before: Option<&'static str>,
    },
    /// Collapse `if (condition) A else B` in type position to `B`.
    TypeConditional,
}

struct Rewrite {
    rule: Rule,
    /// Why the grammar cannot read it.
    reason: &'static str,
    fidelity: Fidelity,
}

/// A keyword blanked wherever it stands alone.
const fn blank(keyword: &'static str, reason: &'static str) -> Rewrite {
    Rewrite {
        rule: Rule::Blank {
            keyword,
            after: None,
            before: None,
        },
        reason,
        fidelity: Fidelity::Preserved,
    }
}

/// A keyword blanked only where the text around it matches.
const fn blank_between(
    keyword: &'static str,
    after: Option<&'static str>,
    before: Option<&'static str>,
    reason: &'static str,
) -> Rewrite {
    Rewrite {
        rule: Rule::Blank {
            keyword,
            after,
            before,
        },
        reason,
        fidelity: Fidelity::Preserved,
    }
}

/// Syntax released grammars do not accept.
const RUST_REWRITES: &[Rewrite] = &[
    blank_between(
        "const",
        None,
        Some("trait"),
        "const traits are unstable, and `trait_item` admits only `unsafe`",
    ),
    blank_between(
        "const",
        Some("impl"),
        None,
        "`impl const Trait for Ty` is unstable",
    ),
    blank("[const]", "conditionally const bounds are unstable"),
    blank(
        "~const",
        "the earlier spelling of conditionally const bounds",
    ),
    blank("become", "explicit tail calls are unstable"),
    blank_between(
        "raw const",
        Some("&"),
        None,
        "raw borrows are unstable; the borrow itself still parses",
    ),
    blank_between(
        "raw mut",
        Some("&"),
        None,
        "raw borrows are unstable; the borrow itself still parses",
    ),
    blank("yield", "generators are unstable"),
    blank_between(
        "try",
        None,
        Some("{"),
        "try blocks are unstable; the block itself still parses",
    ),
    blank_between("auto", None, Some("trait"), "auto traits are unstable"),
];

/// Syntax `tree-sitter-zig` does not accept.
///
/// One entry, because one defect accounts for every occurrence measured: the
/// grammar rejects an `if` expression in type position. It was found through
/// five spellings — `E!if`, `[N]if`, `[]if`, `?if`, and `name: if` — which is
/// why this matches the position rather than the spelling. On Bun v1.3.14 it is
/// what stands between six files and a clean parse.
const ZIG_REWRITES: &[Rewrite] = &[Rewrite {
    rule: Rule::TypeConditional,
    reason: "a comptime-conditional type; the grammar rejects `if` in type position",
    fidelity: Fidelity::Narrowed,
}];

/// The rewrites that apply to a language, if any.
fn table(language: &str) -> &'static [Rewrite] {
    match language {
        "rust" => RUST_REWRITES,
        "zig" => ZIG_REWRITES,
        _ => &[],
    }
}

/// What a rewrite pass changed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rewritten {
    pub source: Vec<u8>,
    /// The reasons that applied, for reporting rather than for silence.
    pub reasons: Vec<&'static str>,
    /// Whether any rewrite changed what the source says rather than only what
    /// the grammar could read. Locating a declaration is unaffected either way;
    /// quoting one is not.
    pub narrowed: bool,
}

/// Blank out syntax the grammar for `language` cannot read.
///
/// Returns `None` when nothing applied, so a caller can tell "no rewrite was
/// needed" from "a rewrite was made".
pub fn neutralize(language: impl AsRef<str>, source: &[u8]) -> Option<Rewritten> {
    let table = table(language.as_ref());
    if table.is_empty() {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?; // straitjacket-allow:error-discard — non-UTF-8 source declares no dialect
    let mut output = text.to_owned();
    let mut reasons = Vec::new();
    let mut narrowed = false;
    for rewrite in table {
        let applied = match rewrite.rule {
            Rule::Blank {
                keyword,
                after,
                before,
            } => blank_all(&mut output, keyword, after, before),
            Rule::TypeConditional => collapse_type_conditionals(&mut output),
        };
        if applied {
            reasons.push(rewrite.reason);
            narrowed |= rewrite.fidelity == Fidelity::Narrowed;
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
        narrowed,
    })
}

/// Replace every qualifying occurrence with spaces, in place.
fn blank_all(text: &mut String, keyword: &str, after: Option<&str>, before: Option<&str>) -> bool {
    let mut blanked = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(keyword) {
        let start = from + offset;
        let end = start + keyword.len();
        from = end;
        if !is_word_boundary(text, start, end) || !context_matches(text, start, end, after, before)
        {
            continue;
        }
        text.replace_range(start..end, &" ".repeat(keyword.len()));
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

fn context_matches(
    text: &str,
    start: usize,
    end: usize,
    after: Option<&str>,
    before: Option<&str>,
) -> bool {
    if let Some(after) = after {
        // `impl<T> const Trait` puts a generic list between the two, so a
        // trailing parameter list is stepped over before the check.
        let preceding = without_trailing_generics(text[..start].trim_end());
        if !preceding.trim_end().ends_with(after) {
            return false;
        }
    }
    if let Some(before) = before
        && !text[end..].trim_start().starts_with(before)
    {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// `if` in type position
// ---------------------------------------------------------------------------

/// Zig words after which a `!` is a negation rather than an error-set operator.
///
/// Without this, `a and !if (windows) x else y` reads as a type. Bun has one,
/// in `resolver.zig`, so the distinction is not hypothetical.
const NEGATION_FOLLOWS: &[&str] = &[
    "and", "or", "return", "if", "while", "for", "else", "try", "comptime", "orelse", "break",
];

/// Collapse every `if (condition) A else B` standing in type position.
///
/// The second branch is the one kept. Either would parse and neither is
/// faithful, so the choice is made once here rather than per site.
fn collapse_type_conditionals(text: &mut String) -> bool {
    let mut collapsed = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find("if") {
        let start = from + offset;
        let after = start + "if".len();
        from = after;
        if !is_word_boundary(text, start, after) || !introduces_a_type(text, start) {
            continue;
        }
        let Some(open) = text[after..].find('(').map(|at| after + at) else {
            continue;
        };
        // Only whitespace may sit between `if` and its condition; anything else
        // means this `if` belongs to something further along.
        if !text[after..open].trim().is_empty() {
            continue;
        }
        let Some(close) = balanced(text, open) else {
            continue;
        };
        // `{` opens a group while the first branch is being crossed, because
        // that branch can be `struct { .. }`, and ends the second, because
        // there it opens the function body.
        let Some(keyword) = scan(text, close, Some("else"), b",;=") else {
            continue;
        };
        let alternative = keyword + "else".len();
        let Some(end) = scan(text, alternative, None, b"{,;=") else {
            continue;
        };
        let kept = text[alternative..end].trim().to_owned();
        let width = end - start;
        if kept.is_empty() {
            continue;
        }
        // Unreachable as written, because the kept branch is part of what it
        // replaces. Kept anyway: it is the only thing between a later edit here
        // and a source that lengthens, and the padded write below would take
        // that silently, moving every span in the file.
        if kept.len() > width {
            continue;
        }
        text.replace_range(start..end, &format!("{kept:<width$}"));
        collapsed = true;
        from = end;
    }
    collapsed
}

/// Whether the `if` at `at` stands where a type belongs, judged by the token
/// that introduces it.
fn introduces_a_type(text: &str, at: usize) -> bool {
    let before = text[..at].trim_end();
    let Some(last) = before.chars().next_back() else {
        return false;
    };
    match last {
        // `[N]if ..`, `[]if ..`, `?if ..`, and `name: if ..`, which covers a
        // variable's type, a struct field's, and a parameter's.
        ']' | '?' | ':' => true,
        // `E!if ..` is an error union. `and !if ..` is a negation and `//! if`
        // is a doc comment, and both appear in Bun.
        '!' => {
            let head = before[..before.len() - '!'.len_utf8()].trim_end();
            match head.chars().next_back() {
                Some(')') => true,
                Some(character) if character.is_alphanumeric() || character == '_' => {
                    !NEGATION_FOLLOWS.contains(&trailing_word(head).as_str())
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// The identifier `text` ends with, if it ends with one.
fn trailing_word(text: &str) -> String {
    let word: String = text
        .chars()
        .rev()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();
    word.chars().rev().collect()
}

/// The byte after the group opened at `open`, or `None` if it never closes.
fn balanced(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, byte) in text.as_bytes()[open..].iter().enumerate() {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Find `needle` at nesting depth zero, giving up at any byte in `stop`.
///
/// With no needle, the position of that stop byte is the answer instead — which
/// is how the end of a branch is found. A stop byte is tested before it is
/// treated as an opener, so `{` can end a return type and still open a group
/// inside one.
fn scan(text: &str, from: usize, needle: Option<&str>, stop: &[u8]) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = from;
    while index < bytes.len() {
        let byte = bytes[index];
        if depth == 0 {
            // Source outside ASCII appears in comments and string literals, and
            // slicing into the middle of one panics.
            if let Some(needle) = needle
                && text.is_char_boundary(index)
                && text[index..].starts_with(needle)
            {
                return Some(index);
            }
            if stop.contains(&byte) {
                return needle.is_none().then_some(index);
            }
        }
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => match depth.checked_sub(1) {
                Some(outer) => depth = outer,
                // A closing bracket we never opened ends whatever we are in.
                None => return needle.is_none().then_some(index),
            },
            _ => {}
        }
        index += 1;
    }
    None
}

/// A prefix with any trailing `<..>` removed, brackets balanced.
fn without_trailing_generics(text: &str) -> &str {
    if !text.ends_with('>') {
        return text;
    }
    let mut depth = 0usize;
    for (index, character) in text.char_indices().rev() {
        match character {
            '>' => depth += 1,
            '<' => {
                depth -= 1;
                if depth == 0 {
                    return &text[..index];
                }
            }
            _ => {}
        }
    }
    text
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
    fn generics_between_impl_and_const_do_not_hide_it() {
        let rewritten = rewrite("impl<T> const IntoIterator for Option<T> {}").unwrap();
        assert!(!rewritten.contains("const"), "{rewritten}");
        assert_eq!(
            rewritten.len(),
            "impl<T> const IntoIterator for Option<T> {}".len()
        );
        // nested generics too
        assert!(
            !rewrite("impl<T: Into<U>, U> const Foo for T {}")
                .unwrap()
                .contains("const")
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

    #[test]
    fn a_blanking_rewrite_does_not_narrow_anything() {
        let rewritten = neutralize("rust", b"pub const trait T {}").expect("rewritten");
        assert!(!rewritten.narrowed);
    }

    // -- Zig: `if` in type position -----------------------------------------
    //
    // Every source line below is from Bun v1.3.14, because a rewrite that only
    // works on invented examples is how the first version of this got written.

    fn zig(source: &str) -> Option<String> {
        neutralize("zig", source.as_bytes())
            .map(|rewritten| String::from_utf8(rewritten.source).unwrap_or_default())
    }

    #[test]
    fn every_spelling_of_a_conditional_type_is_collapsed() {
        // error union — cli/pack_command.zig
        assert!(
            zig(") PackError(for_publish)!if (for_publish) Publish.Context(true) else void {")
                .expect("collapsed")
                .starts_with(") PackError(for_publish)!void")
        );
        // array — patch/patch.zig
        assert!(
            zig(") [2]if (sentinel) [:0]const u8 else []const u8 {")
                .expect("collapsed")
                .starts_with(") [2][]const u8")
        );
        // slice, as a struct field — install/PackageInstall.zig. Not in
        // cowbird's patch table, which is how this rule earns its keep.
        assert!(
            zig("to_copy_into2: []if (Environment.isWindows) u16 else u8,")
                .expect("collapsed")
                .starts_with("to_copy_into2: []u8")
        );
        // optional — runtime/node/node_fs.zig
        assert!(
            zig("reuse_stat: ?if (Environment.isWindows) windows.DWORD else std.posix.Stat,")
                .expect("collapsed")
                .starts_with("reuse_stat: ?std.posix.Stat")
        );
        // a bare type annotation — install/extract_tarball.zig
        assert!(
            zig(
                "var b: if (Environment.isWindows) bun.WPathBuffer else bun.PathBuffer = undefined;"
            )
            .expect("collapsed")
            .starts_with("var b: bun.PathBuffer")
        );
    }

    #[test]
    fn a_collapse_never_moves_a_byte() {
        let source = "reuse_stat: ?if (Environment.isWindows) windows.DWORD else std.posix.Stat,";
        assert_eq!(zig(source).expect("collapsed").len(), source.len());
    }

    #[test]
    fn a_collapse_says_it_narrowed_the_source() {
        let rewritten = neutralize("zig", b"x: if (a) u16 else u8,").expect("collapsed");
        assert!(
            rewritten.narrowed,
            "a discarded branch is not a faithful signature"
        );
    }

    #[test]
    fn a_first_branch_containing_braces_does_not_hide_the_else() {
        // runtime/node/node_crypto_binding.zig. The `,` inside the anonymous
        // struct would end the scan if `{` were not treated as an opener.
        assert!(
            zig(") JSError!if (is_async) struct { @This(), JSValue } else @This() {")
                .expect("collapsed")
                .starts_with(") JSError!@This()")
        );
    }

    #[test]
    fn an_if_in_value_position_is_left_alone() {
        // `!` after a keyword is a negation — resolver/resolver.zig.
        assert!(zig("const x = a and !if (Environment.isWindows) b else c;").is_none());
        // A doc comment — threading/Mutex.zig.
        assert!(zig("//! if (m.tryLock()) {").is_none());
        // A default value, not a type. Both halves of
        // install/lockfile/Tree.zig's field are here: the type IS collapsed and
        // the initializer is NOT.
        let both = "f: if (m == .filter) []const W else void = if (m == .filter) &.{},";
        let rewritten = zig(both).expect("the type is collapsed");
        assert!(rewritten.starts_with("f: void"), "{rewritten}");
        assert!(
            rewritten.contains("= if (m == .filter) &.{},"),
            "the initializer is a value, not a type: {rewritten}"
        );
    }

    #[test]
    fn ordinary_zig_is_left_alone() {
        assert!(zig("if (a) { b(); } else { c(); }").is_none());
        assert!(zig("const x = 1;").is_none());
        // `if` inside a longer identifier.
        assert!(zig("fn notify(self: *Self) void {}").is_none());
        assert!(zig("pub fn f(x: u32) !void { return; }").is_none());
    }

    #[test]
    fn an_unterminated_conditional_is_refused_rather_than_guessed() {
        // No `else`, so there is no second branch to keep.
        assert!(zig("x: if (a) u16,").is_none());
        // No closing paren.
        assert!(zig("x: if (a u16 else u8,").is_none());
        // An empty second branch leaves nothing to keep.
        assert!(zig("x: if (a) u16 else ,").is_none());
        // The kept branch is always part of what it replaces, so it always
        // fits, however long it is. This is what makes the padded write safe.
        let long = "x: if (a) u8 else SomeVeryLongTypeName,";
        let rewritten = zig(long).expect("collapsed");
        assert_eq!(rewritten.len(), long.len());
        assert!(
            rewritten.starts_with("x: SomeVeryLongTypeName"),
            "{rewritten}"
        );
    }

    #[test]
    fn rust_is_unaffected_by_the_zig_rule() {
        // `:` before an `if` is a type annotation in Zig and never in Rust, so
        // the two tables must not be shared.
        assert!(neutralize("rust", b"let x: u32 = if (a) { 1 } else { 2 };").is_none());
    }
}
