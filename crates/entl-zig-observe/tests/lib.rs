// Tests for `src/lib.rs`: the crate's public surface.
//
// Three modules — shapes, functions, assignments — and the span they all carry.
// The span's lines are one-based, which is the part worth stating: it matches
// what an editor and `file:line` evidence both mean, and an off-by-one here
// points every reader at the wrong line.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_zig_observe::Span;

#[test]
fn a_span_carries_bytes_and_one_based_lines() {
    let span = Span {
        start_byte: 0,
        end_byte: 4,
        start_line: 1,
        end_line: 1,
    };
    assert_eq!(span.end_byte - span.start_byte, 4);
    assert_eq!(span.start_line, 1, "lines are one-based");
}
