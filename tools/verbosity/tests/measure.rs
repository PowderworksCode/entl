// Tests for `src/measure.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::CommentSyntax;
use entl::codebase::comment_syntax;
use verbosity::measure::*;

fn syntax(language: &str) -> &'static CommentSyntax {
    comment_syntax(language).expect("language has comment syntax")
}

#[test]
fn drops_line_comments_and_blank_lines() {
    let measured = measure("// header\n\nint main() {}\n", syntax("c"));
    assert_eq!(
        measured,
        Measurement {
            lines: 1,
            bytes: 13
        }
    );
}

#[test]
fn keeps_code_split_across_a_block_comment() {
    let measured = measure("a;\n/* note\n   more */\nb;\n", syntax("c"));
    assert_eq!(measured, Measurement { lines: 2, bytes: 4 });
}

#[test]
fn ignores_comment_markers_inside_strings() {
    let measured = measure("print(\"# not a comment\")\n", syntax("python"));
    assert_eq!(
        measured,
        Measurement {
            lines: 1,
            bytes: 24
        }
    );
}

#[test]
fn treats_python_triple_quotes_as_content() {
    let measured = measure("x = \"\"\"a\nb\"\"\"\n", syntax("python"));
    assert_eq!(
        measured,
        Measurement {
            lines: 2,
            bytes: 12
        }
    );
}

#[test]
fn survives_rust_lifetimes() {
    let measured = measure("fn f<'a>(x: &'a str) {}\nlet y = 1;\n", syntax("rust"));
    assert_eq!(measured.lines, 2);
}

#[test]
fn strips_indentation_but_not_inner_spacing() {
    let measured = measure("        if x == 1:\n", syntax("python"));
    assert_eq!(
        measured,
        Measurement {
            lines: 1,
            bytes: 10
        }
    );
}
