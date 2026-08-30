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
        /// Whether the keyword must be the first thing on its line.
        ///
        /// A soft keyword is an ordinary identifier everywhere else, so
        /// position is the only thing separating the two. `lazy import x` is
        /// PEP 810; `from .lazy import x` names a module called `lazy`, and
        /// blanking that one turns a file that parses into one that does not.
        /// CPython's own `test_syntax.py` holds both.
        leading: bool,
    },
    /// Collapse `if (condition) A else B` in type position to `B`.
    TypeConditional,
    /// Rewrite a statement-position macro call so the grammar reads a
    /// conditional: `for_each_x(a, b) {` becomes `if(a, b) {`, the head
    /// overwritten by spaces that end in `if`.
    ///
    /// C spells custom control flow as a function-like macro whose expansion
    /// is a `for` header, and a grammar that never sees the expansion reads a
    /// call expression that forgot its semicolon. The arguments stay exactly
    /// where they were — `(a, b)` is a parenthesized comma expression, which
    /// `if` accepts — so every identifier keeps its offset and stays visible
    /// to a consumer reading references. What changes is the shape: a loop
    /// now reads as a conditional, which is why this is `Narrowed`.
    CallToIf {
        /// The macro head. At least three bytes, or `if` would not fit.
        keyword: &'static str,
    },
    /// Blank a whole macro invocation — head through balanced `)`, a
    /// preceding `static`, a trailing `;` — keeping every newline in place.
    ///
    /// For macros that *generate declarations* (`define_commit_slab`,
    /// `GIT_PATH_FUNC`): no rearrangement of the unexpanded text is a C
    /// declaration, so the only length-preserving way to recover the file is
    /// to remove the invocation entirely. The declarations the macro would
    /// have generated were never visible to a syntax reader in the first
    /// place; what is lost is the *site*, which is why this is `Narrowed`.
    BlankInvocation {
        /// The macro head.
        keyword: &'static str,
    },
    /// Inside `keyword(..)`, overwrite each type-name argument with `0`.
    ///
    /// `container_of(ptr, struct foo, member)` passes a *type* where the
    /// grammar expects an expression. `0` is an expression, fits in one byte,
    /// and the rest of the argument becomes padding.
    TypeArgZero {
        /// The macro head whose arguments may name types.
        keyword: &'static str,
    },
    /// Inside `keyword(..)`, overwrite the argument at `index` with `0`,
    /// whatever it says.
    ///
    /// For macros whose signature *fixes* which argument is a type —
    /// `va_arg(ap, const char *)` — where recognizing the type by its
    /// spelling would be guesswork and the position is certain.
    ZeroArg {
        /// The macro head.
        keyword: &'static str,
        /// Zero-based argument position to overwrite.
        index: usize,
    },
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
            leading: false,
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
            leading: false,
        },
        reason,
        fidelity: Fidelity::Preserved,
    }
}

/// A keyword blanked only where it opens a line and `before` follows it.
///
/// For a soft keyword, which is an ordinary identifier in every other position.
const fn blank_leading(
    keyword: &'static str,
    before: &'static str,
    reason: &'static str,
) -> Rewrite {
    Rewrite {
        rule: Rule::Blank {
            keyword,
            after: None,
            before: Some(before),
            leading: true,
        },
        reason,
        fidelity: Fidelity::Preserved,
    }
}

/// A statement-position macro call rewritten to a conditional.
const fn call_to_if(keyword: &'static str, reason: &'static str) -> Rewrite {
    Rewrite {
        rule: Rule::CallToIf { keyword },
        reason,
        fidelity: Fidelity::Narrowed,
    }
}

/// A macro invocation blanked in full, wherever it appears.
const fn blank_invocation(keyword: &'static str, reason: &'static str) -> Rewrite {
    Rewrite {
        rule: Rule::BlankInvocation { keyword },
        reason,
        fidelity: Fidelity::Narrowed,
    }
}

/// A macro whose type-name arguments are each overwritten with `0`.
const fn type_arg_zero(keyword: &'static str, reason: &'static str) -> Rewrite {
    Rewrite {
        rule: Rule::TypeArgZero { keyword },
        reason,
        fidelity: Fidelity::Narrowed,
    }
}

/// A macro whose argument at a fixed position is overwritten with `0`.
const fn zero_arg(keyword: &'static str, index: usize, reason: &'static str) -> Rewrite {
    Rewrite {
        rule: Rule::ZeroArg { keyword, index },
        reason,
        fidelity: Fidelity::Narrowed,
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

/// Syntax `tree-sitter-python` 0.25.0 does not accept.
///
/// PEP 810's lazy imports, in both spellings. Measured on CPython `main` at
/// 2ba0b2c9d1d0: 55 of the 61 files the grammar rejects contain one, and no
/// other single construct accounts for more than one file.
///
/// The cost is smaller here than the Zig entry's, and worth stating plainly:
/// Python statements are delimited by newlines, so an unreadable one takes
/// itself and not the file. Across CPython `main` the grammar loses 0.055% of
/// its bytes. What it loses is nonetheless an `import`, and a consumer reading
/// dependencies from a tree cannot tell a module that imports nothing from one
/// whose imports it could not read.
///
/// `lazy` is a SOFT keyword — an ordinary identifier in every other position —
/// which is why both entries are anchored to the start of a line.
const PYTHON_REWRITES: &[Rewrite] = &[
    blank_leading(
        "lazy",
        "import",
        "PEP 810 lazy imports; the grammar predates them",
    ),
    blank_leading(
        "lazy",
        "from",
        "PEP 810 lazy imports; the grammar predates them",
    ),
];

/// C that `tree-sitter-c` cannot read without knowing the macros.
///
/// C is the language where the text is not the program: the preprocessor sits
/// between them. Measured on git at `5b24717` (825 product C/H files,
/// `tree-sitter-c` 0.24.2), the gap is nonetheless narrow and almost entirely
/// nameable — the grammar returns a tree for every file, 59.4% of them clean,
/// and three macro idioms account for most of the rest:
///
/// * **Attribute macros in declarator position.** `void f(UNUSED int x)` is
///   unreadable to a grammar that has never heard of `UNUSED`. Blanked;
///   nothing the declaration says changes. The six git spellings alone took
///   ERROR nodes from 1,486 to 355 on that corpus.
/// * **Iterator macros in statement position.** `for_each_string_list_item`
///   and eleven relatives read as calls that forgot their semicolon. Nineteen
///   distinct heads accounted for every one of the 569 `MISSING ;` sites.
///   Rewritten to `if`, which keeps every argument identifier in place.
/// * **Declaration-generating macros at file scope.** `define_commit_slab`,
///   `GIT_PATH_FUNC`, X-macro lists. Nothing readable stands in their place,
///   so the invocation is blanked in full.
///
/// These names are git's. That is by design, not oversight: a dialect table
/// accretes per corpus (the Rust table above exists because of what `core`
/// writes), and git is the corpus C support was built against. Other corpora
/// will add their own heads; the rules are the reusable part.
const C_REWRITES: &[Rewrite] = &[
    // -- attribute macros, blanked: the declaration still says what it said.
    blank("UNUSED", "git's parameter attribute; declarator position"),
    blank("MAYBE_UNUSED", "git's declaration attribute"),
    blank("NORETURN_PTR", "git's noreturn for function pointers"),
    blank("NORETURN", "git's noreturn attribute; declarator position"),
    blank("LAST_ARG_MUST_BE_NULL", "git's sentinel attribute"),
    blank("RESULT_MUST_BE_USED", "git's warn_unused_result attribute"),
    blank("REFTABLE_UNUSED", "reftable's postfix parameter attribute"),
    blank("WINAPI", "the win32 calling-convention macro"),
    blank("NTAPI", "the win32 calling-convention macro"),
    // -- iterator macros, rewritten to a conditional the grammar can read.
    call_to_if(
        "for_each_string_list_item",
        "git's string-list iterator macro; a for header in disguise",
    ),
    call_to_if(
        "strmap_for_each_entry",
        "git's strmap iterator macro; a for header in disguise",
    ),
    call_to_if(
        "strintmap_for_each_entry",
        "git's strintmap iterator macro; a for header in disguise",
    ),
    call_to_if(
        "strset_for_each_entry",
        "git's strset iterator macro; a for header in disguise",
    ),
    call_to_if(
        "hashmap_for_each_entry_from",
        "git's hashmap iterator macro; a for header in disguise",
    ),
    call_to_if(
        "hashmap_for_each_entry",
        "git's hashmap iterator macro; a for header in disguise",
    ),
    call_to_if(
        "list_for_each_safe",
        "git's list iterator macro; a for header in disguise",
    ),
    call_to_if(
        "list_for_each_prev",
        "git's list iterator macro; a for header in disguise",
    ),
    call_to_if(
        "list_for_each_dir",
        "git's list iterator macro; a for header in disguise",
    ),
    call_to_if(
        "list_for_each",
        "git's list iterator macro; a for header in disguise",
    ),
    call_to_if(
        "prio_queue_for_each",
        "git's priority-queue iterator macro; a for header in disguise",
    ),
    call_to_if(
        "repo_for_each_pack",
        "git's pack iterator macro; a for header in disguise",
    ),
    call_to_if(
        "for_each_wanted_builtin",
        "trace2's target iterator macro; a for header in disguise",
    ),
    call_to_if(
        "for_each_builtin",
        "trace2's target iterator macro; a for header in disguise",
    ),
    // -- declaration-generating macros, blanked in full at their site.
    blank_invocation(
        "define_commit_slab",
        "generates a struct and functions no syntax reader sees",
    ),
    blank_invocation(
        "declare_commit_slab",
        "generates declarations no syntax reader sees",
    ),
    blank_invocation(
        "define_shared_commit_slab",
        "generates declarations no syntax reader sees",
    ),
    blank_invocation(
        "implement_shared_commit_slab",
        "generates definitions no syntax reader sees",
    ),
    blank_invocation(
        "implement_commit_slab",
        "generates definitions no syntax reader sees",
    ),
    blank_invocation(
        "DEFINE_LIST_SORT_DEBUG",
        "generates a sort function no syntax reader sees",
    ),
    blank_invocation(
        "DEFINE_LIST_SORT",
        "generates a sort function no syntax reader sees",
    ),
    blank_invocation(
        "DECLARE_LIST_SORT",
        "generates a prototype no syntax reader sees",
    ),
    blank_invocation(
        "REPO_GIT_PATH_FUNC",
        "generates an accessor function no syntax reader sees",
    ),
    blank_invocation(
        "GIT_PATH_FUNC",
        "generates an accessor function no syntax reader sees",
    ),
    blank_invocation(
        "KHASH_INIT",
        "khash's hash-table generator; a header's worth of definitions",
    ),
    blank_invocation(
        "FOREACH_FSCK_MSG_ID",
        "an X-macro list expanded inside an enum body",
    ),
    blank_invocation(
        "DECLARE_PROC_ADDR",
        "win32 dynamic-symbol declaration macro",
    ),
    blank_invocation("libc_hidden_def", "a glibc visibility annotation"),
    blank_invocation("strong_alias", "a glibc alias annotation"),
    blank_invocation(
        "SHA1_STORE_STATE",
        "sha1dc's unrolled statement macro; no semicolon follows",
    ),
    blank_invocation(
        "SHA1_RECOMPRESS",
        "sha1dc's unrolled statement macro; no semicolon follows",
    ),
    blank_invocation(
        "FORMAT_PRESERVING",
        "git's gettext format attribute; declarator position",
    ),
    // -- macros taking a type where the grammar expects an expression.
    type_arg_zero(
        "hashmap_clear_and_free",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "hashmap_partial_clear_and_free",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "hashmap_iter_first_entry",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "hashmap_get_entry_from_hash",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "hashmap_get_entry",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "list_first_entry",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "list_last_entry",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "container_of_or_null",
        "takes a type argument in expression position",
    ),
    type_arg_zero(
        "container_of",
        "takes a type argument in expression position",
    ),
    type_arg_zero("list_entry", "takes a type argument in expression position"),
    zero_arg(
        "va_arg",
        1,
        "its second argument is a type by definition; position is certain",
    ),
    zero_arg(
        "maximum_unsigned_value_of_type",
        0,
        "its only argument is a type by definition; position is certain",
    ),
    // -- win32 SAL annotations and one macro-pasted string pair.
    blank("_In_opt_", "a win32 SAL parameter annotation"),
    blank("_In_", "a win32 SAL parameter annotation"),
    blank("_Reserved_", "a win32 SAL parameter annotation"),
    blank_between(
        "DISPLAY_PREFIX",
        Some("ANSI_PREFIX"),
        None,
        "two macro string constants juxtaposed; one suffices to parse",
    ),
    // -- heads measured on curl and redis, the two calibration corpora.
    //
    // The rule *kinds* above covered every failure class on both; only these
    // names were new. That is the pattern to expect: onboarding a corpus is a
    // matter of heads, not of mechanisms.
    blank(
        "UNITTEST_BEGIN_SIMPLE",
        "curl's unit-test opener, used bare with no arguments",
    ),
    blank_invocation(
        "UNITTEST_BEGIN",
        "curl's unit-test opener; a block header in disguise",
    ),
    blank("UNITTEST_END_SIMPLE", "curl's unit-test closer"),
    blank_invocation("UNITTEST_END", "curl's unit-test closer"),
    blank("UNITTEST_STOP", "curl's unit-test closer"),
    blank(
        "JEMALLOC_ALWAYS_INLINE",
        "jemalloc's attribute macro before a return type",
    ),
    blank(
        "JEMALLOC_NOINLINE",
        "jemalloc's attribute macro before a return type",
    ),
    blank("JEMALLOC_EXPORT", "jemalloc's visibility macro"),
    blank("TEST_END", "jemalloc's unit-test closer"),
    blank_invocation("TEST_BEGIN", "jemalloc's unit-test opener"),
];

/// The rewrites that apply to a language, if any.
fn table(language: &str) -> &'static [Rewrite] {
    match language {
        "c" => C_REWRITES,
        "python" => PYTHON_REWRITES,
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
                leading,
            } => blank_all(&mut output, keyword, after, before, leading),
            Rule::TypeConditional => collapse_type_conditionals(&mut output),
            Rule::CallToIf { keyword } => call_to_if_all(&mut output, keyword),
            Rule::BlankInvocation { keyword } => blank_invocation_all(&mut output, keyword),
            Rule::TypeArgZero { keyword } => type_arg_zero_all(&mut output, keyword),
            Rule::ZeroArg { keyword, index } => zero_arg_all(&mut output, keyword, index),
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
fn blank_all(
    text: &mut String,
    keyword: &str,
    after: Option<&str>,
    before: Option<&str>,
    leading: bool,
) -> bool {
    let mut blanked = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(keyword) {
        let start = from + offset;
        let end = start + keyword.len();
        from = end;
        if leading && !opens_a_line(text, start) {
            continue;
        }
        if !is_word_boundary(text, start, end) || !context_matches(text, start, end, after, before)
        {
            continue;
        }
        // A corpus can define a macro of the same name (`#define UNUSED(x)
        // (void)(x)` — redis does): blanking the name out of its own
        // definition guarantees the retry fails. Skipped on directive lines
        // for every language; none of them writes a rewrite target there.
        if on_a_preproc_line(text, start) {
            continue;
        }
        text.replace_range(start..end, &" ".repeat(keyword.len()));
        blanked = true;
    }
    blanked
}

/// Rewrite every statement-position `keyword(..)` to `if(..)`, in place.
///
/// Statement position is judged the soft-keyword way: the head must open its
/// line. Every measured use does (an iterator macro is a statement, and C
/// indents statements), and the restriction is what keeps the macro's own
/// `#define` line untouched — there the head follows `#define `, not
/// indentation.
fn call_to_if_all(text: &mut String, keyword: &str) -> bool {
    debug_assert!(keyword.len() >= "if".len());
    let mut rewritten = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(keyword) {
        let start = from + offset;
        let end = start + keyword.len();
        from = end;
        if !is_word_boundary(text, start, end) || !opens_a_line(text, start) {
            continue;
        }
        // The parenthesis must follow on the same line; only spacing between.
        if !text[end..].trim_start_matches([' ', '\t']).starts_with('(') {
            continue;
        }
        let pad = keyword.len() - "if".len();
        text.replace_range(start..end, &format!("{}if", " ".repeat(pad)));
        rewritten = true;
    }
    rewritten
}

/// Blank every `keyword(..)` invocation in full, in place.
///
/// The span runs from a preceding `static` (when one abuts) through the
/// balanced close parenthesis and a trailing `;` (when one abuts). Newlines
/// inside the span stay, so every later line keeps its number; every other
/// character becomes the spaces its width requires.
fn blank_invocation_all(text: &mut String, keyword: &str) -> bool {
    let mut rewritten = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(keyword) {
        let head = from + offset;
        let head_end = head + keyword.len();
        from = head_end;
        if !is_word_boundary(text, head, head_end) || on_a_preproc_line(text, head) {
            continue;
        }
        let after_head = &text[head_end..];
        let spaces = after_head.len() - after_head.trim_start_matches([' ', '\t']).len();
        let open = head_end + spaces;
        if !text[open..].starts_with('(') {
            continue;
        }
        let Some(close) = balanced(text, open) else {
            continue;
        };
        // A `static` immediately before belongs to the declaration the macro
        // would have generated; left behind it declares nothing and fails.
        let mut start = head;
        let preceding = text[..head].trim_end_matches([' ', '\t']);
        if preceding.ends_with("static") {
            let candidate = preceding.len() - "static".len();
            if is_word_boundary(text, candidate, preceding.len()) {
                start = candidate;
            }
        }
        let mut end = close;
        let trailing = &text[close..];
        let gap = trailing.len() - trailing.trim_start_matches([' ', '\t']).len();
        if text[close + gap..].starts_with(';') {
            end = close + gap + ';'.len_utf8();
        }
        let blanked: String = text[start..end]
            .chars()
            .map(|character| {
                if character == '\n' || character == '\r' {
                    character.to_string()
                } else {
                    " ".repeat(character.len_utf8())
                }
            })
            .collect();
        text.replace_range(start..end, &blanked);
        rewritten = true;
        from = end;
    }
    rewritten
}

/// In every `keyword(..)`, overwrite each type-name argument with `0`, padded.
///
/// An argument is a type name when it reads as `struct tag`, `union tag`, or
/// `enum tag`, optionally `const`-qualified, optionally pointed to. `0` is the
/// shortest expression there is, and the argument's remaining width becomes
/// spaces, so nothing moves.
fn type_arg_zero_all(text: &mut String, keyword: &str) -> bool {
    let mut rewritten = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(keyword) {
        let head = from + offset;
        let head_end = head + keyword.len();
        from = head_end;
        if !is_word_boundary(text, head, head_end) || on_a_preproc_line(text, head) {
            continue;
        }
        let after_head = &text[head_end..];
        let spaces = after_head.len() - after_head.trim_start_matches([' ', '\t']).len();
        let open = head_end + spaces;
        if !text[open..].starts_with('(') {
            continue;
        }
        let Some(close) = balanced(text, open) else {
            continue;
        };
        for (argument_start, argument_end) in argument_spans(text, open, close) {
            if !names_a_type(text[argument_start..argument_end].trim()) {
                continue;
            }
            let replacement: String = text[argument_start..argument_end]
                .char_indices()
                .map(|(at, character)| {
                    if at == 0 {
                        "0".to_string()
                    } else if character == '\n' || character == '\r' {
                        character.to_string()
                    } else {
                        " ".repeat(character.len_utf8())
                    }
                })
                .collect();
            text.replace_range(argument_start..argument_end, &replacement);
            rewritten = true;
        }
        from = close;
    }
    rewritten
}

/// In every `keyword(..)`, overwrite the argument at `index` with `0`, padded.
fn zero_arg_all(text: &mut String, keyword: &str, index: usize) -> bool {
    let mut rewritten = false;
    let mut from = 0;
    while let Some(offset) = text[from..].find(keyword) {
        let head = from + offset;
        let head_end = head + keyword.len();
        from = head_end;
        if !is_word_boundary(text, head, head_end) || on_a_preproc_line(text, head) {
            continue;
        }
        let after_head = &text[head_end..];
        let spaces = after_head.len() - after_head.trim_start_matches([' ', '\t']).len();
        let open = head_end + spaces;
        if !text[open..].starts_with('(') {
            continue;
        }
        let Some(close) = balanced(text, open) else {
            continue;
        };
        let spans = argument_spans(text, open, close);
        let Some(&(argument_start, argument_end)) = spans.get(index) else {
            continue;
        };
        // Already a lone `0` (a second pass, or source that was cheap to begin
        // with): nothing to change, and claiming a rewrite would be a lie.
        if text[argument_start..argument_end].trim() == "0" {
            continue;
        }
        let replacement: String = text[argument_start..argument_end]
            .char_indices()
            .map(|(at, character)| {
                if at == 0 {
                    "0".to_string()
                } else if character == '\n' || character == '\r' {
                    character.to_string()
                } else {
                    " ".repeat(character.len_utf8())
                }
            })
            .collect();
        text.replace_range(argument_start..argument_end, &replacement);
        rewritten = true;
        from = close;
    }
    rewritten
}

/// The comma-separated argument spans between `open`'s `(` and `close`,
/// trimmed of leading whitespace so a replacement can start at a character.
fn argument_spans(text: &str, open: usize, close: usize) -> Vec<(usize, usize)> {
    let inner_start = open + '('.len_utf8();
    let inner_end = close - ')'.len_utf8();
    let mut spans = Vec::new();
    let mut depth = 0usize;
    let mut argument_start = inner_start;
    for (offset, byte) in text.as_bytes()[inner_start..inner_end].iter().enumerate() {
        let at = inner_start + offset;
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                spans.push((argument_start, at));
                argument_start = at + 1;
            }
            _ => {}
        }
    }
    spans.push((argument_start, inner_end));
    spans
        .into_iter()
        .map(|(start, end)| {
            let trimmed = start + (text[start..end].len() - text[start..end].trim_start().len());
            (trimmed, end)
        })
        .filter(|(start, end)| start < end)
        .collect()
}

/// Whether a trimmed argument reads as a C type name: `struct tag`,
/// `union tag`, or `enum tag`, optionally `const`-qualified before, optionally
/// starred after.
fn names_a_type(argument: &str) -> bool {
    let argument = argument.strip_prefix("const").unwrap_or(argument).trim();
    let Some(rest) = ["struct", "union", "enum"]
        .iter()
        .find_map(|introducer| argument.strip_prefix(introducer))
    else {
        return false;
    };
    // The introducer must be a word of its own: `structural` names no struct.
    let Some(first) = rest.chars().next() else {
        return false;
    };
    if !first.is_whitespace() {
        return false;
    }
    let tag = rest.trim();
    !tag.is_empty()
        && tag.chars().all(|character| {
            character.is_alphanumeric() || character == '_' || character == '*' || character == ' '
        })
}

/// Whether the occurrence at `at` sits on a preprocessor line.
///
/// A macro's own `#define` mentions the same head the sites do; blanking the
/// name out of a `#define` leaves a directive with nothing to define, which
/// turns a readable line into a failure. Sites on continuation lines are not
/// caught — those lines do not open with `#` — and do not need to be: inside
/// a directive's body the grammar reads token soup and no rewrite is needed.
fn on_a_preproc_line(text: &str, at: usize) -> bool {
    text[..at]
        .rsplit_once('\n')
        .map_or(&text[..at], |(_, line)| line)
        .trim_start()
        .starts_with('#')
}

/// Whether only indentation separates the occurrence at `at` from the start of
/// its line.
fn opens_a_line(text: &str, at: usize) -> bool {
    text[..at]
        .rsplit_once('\n')
        .map_or(&text[..at], |(_, line)| line)
        .chars()
        .all(|character| character == ' ' || character == '\t')
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
