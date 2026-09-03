// Tests for `src/dialect.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_tree_sitter::*;

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
        zig("var b: if (Environment.isWindows) bun.WPathBuffer else bun.PathBuffer = undefined;")
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

// -- Python: PEP 810 lazy imports ---------------------------------------

fn python(source: &str) -> Option<String> {
    neutralize("python", source.as_bytes())
        .map(|rewritten| String::from_utf8(rewritten.source).unwrap_or_default())
}

#[test]
fn both_spellings_of_a_lazy_import_are_blanked() {
    assert_eq!(python("lazy import os\n").unwrap(), "     import os\n");
    assert_eq!(
        python("lazy from os import path\n").unwrap(),
        "     from os import path\n"
    );
    // Indented, as in `Lib/concurrent/futures/__init__.py`.
    assert_eq!(
        python("    lazy from .a import B\n").unwrap(),
        "         from .a import B\n"
    );
}

#[test]
fn a_lazy_rewrite_never_moves_a_byte() {
    let source = "lazy import json\nx = 1\n";
    assert_eq!(python(source).expect("blanked").len(), source.len());
}

#[test]
fn blanking_a_lazy_import_does_not_narrow_anything() {
    let rewritten = neutralize("python", b"lazy import os\n").expect("blanked");
    assert!(
        !rewritten.narrowed,
        "the module is still imported and the name still bound"
    );
}

/// A soft keyword is an ordinary identifier everywhere but one position,
/// and `from .lazy import x` turns a file that PARSES into one that does
/// not. CPython's `Lib/test/test_syntax.py` holds this and a real lazy
/// import in the same file.
#[test]
fn lazy_is_only_a_keyword_where_it_opens_a_line() {
    assert!(python("from .lazy import x\n").is_none());
    assert!(python("from ...lazy import x\n").is_none());
    assert!(python("from . sub.lazy import x\n").is_none());
    assert!(python("import lazy\n").is_none());
    assert!(python("x = lazy\nimport os\n").is_none());
    // A longer word that merely starts with it.
    assert!(python("lazy_import(module)\n").is_none());
    assert!(python("lazily import\n").is_none());
}

#[test]
fn ordinary_python_is_left_alone() {
    assert!(python("import os\n").is_none());
    assert!(python("from collections import defaultdict\n").is_none());
    assert!(python("def f():\n    pass\n").is_none());
}

#[test]
fn other_languages_do_not_take_the_python_rule() {
    // `lazy` opens a line before an `import` in TypeScript too, and there
    // it is ordinary code.
    assert!(neutralize("typescript", b"lazy\nimport x from 'y';").is_none());
}

#[test]
fn rust_is_unaffected_by_the_zig_rule() {
    // `:` before an `if` is a type annotation in Zig and never in Rust, so
    // the two tables must not be shared.
    assert!(neutralize("rust", b"let x: u32 = if (a) { 1 } else { 2 };").is_none());
}

fn c(source: &str) -> Option<Rewritten> {
    neutralize("c", source.as_bytes())
}

#[test]
fn c_attribute_macros_blank_in_place() {
    let rewritten = c("int f(UNUSED int x);\n").expect("applies");
    assert_eq!(rewritten.source, b"int f(       int x);\n");
    assert!(!rewritten.narrowed);
}

#[test]
fn c_iterator_macros_become_conditionals() {
    let rewritten =
        c("\tfor_each_string_list_item(item, &list) {\n\t\tuse(item);\n\t}\n").expect("applies");
    assert_eq!(
        rewritten.source,
        b"\t                       if(item, &list) {\n\t\tuse(item);\n\t}\n"
    );
    assert!(rewritten.narrowed);
}

#[test]
fn c_iterator_macro_definitions_are_left_alone() {
    // The `#define` line mentions the same head every site does; the site
    // rewrites, the definition must not.
    assert!(c("#define for_each_string_list_item(i, l) for (...)\n").is_none());
}

/// Every byte space or newline, length unchanged, newlines where they were.
fn blanked_in_place(original: &str, rewritten: &[u8]) {
    assert_eq!(rewritten.len(), original.len());
    for (index, byte) in rewritten.iter().enumerate() {
        match original.as_bytes()[index] {
            b'\n' => assert_eq!(*byte, b'\n'),
            _ => assert_eq!(*byte, b' '),
        }
    }
}

#[test]
fn c_declaration_macros_blank_in_full() {
    let source = "define_commit_slab(indegree, int);\n";
    blanked_in_place(source, &c(source).expect("applies").source);
    // `static` in front belongs to the generated declaration and goes too.
    let with_static = "static GIT_PATH_FUNC(git_path_x, \"X\")\n";
    blanked_in_place(with_static, &c(with_static).expect("applies").source);
}

#[test]
fn c_multi_line_invocations_keep_their_newlines() {
    let source = "KHASH_INIT(str, const char *,\n\tvoid *, 1, h, eq)\n";
    blanked_in_place(source, &c(source).expect("applies").source);
}

#[test]
fn c_type_arguments_become_zero() {
    let rewritten = c("e = container_of(ptr, const struct entry, member);\n").expect("applies");
    assert_eq!(
        rewritten.source,
        b"e = container_of(ptr, 0                 , member);\n"
    );
    // An argument that is already an expression is left alone.
    assert!(c("e = container_of(ptr, entry, member);\n").is_none());
}

#[test]
fn c_va_arg_zeroes_by_position_not_spelling() {
    let rewritten = c("s = va_arg(ap, const char *);\n").expect("applies");
    assert_eq!(rewritten.source, b"s = va_arg(ap, 0           );\n");
}

#[test]
fn c_macro_definitions_of_blanked_names_are_left_alone() {
    // redis defines its own `UNUSED(x)`; the name must survive in its own
    // `#define` or the file trades one failure for another.
    assert!(c("#define UNUSED(x) (void)(x)\n").is_none());
}

#[test]
fn ordinary_c_is_left_alone() {
    assert!(c("int main(void) { return 0; }\n").is_none());
    assert!(c("struct entry { int x; };\n").is_none());
}
