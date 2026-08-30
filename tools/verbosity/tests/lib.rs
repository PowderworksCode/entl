// Tests for `src/lib.rs`: the four modules the binary is a front over.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use verbosity::{corpus, emit, measure, stats};

/// Naming them is the test: main.rs binds to all four, so one made private
/// breaks the binary rather than this file.
#[test]
fn every_module_is_public() {
    let _ = corpus::Source::Exercism;
    let _: fn(&emit::Report) -> String = emit::table;
    let _: fn(&str, &entl::codebase::CommentSyntax) -> measure::Measurement = measure::measure;
    let _ = stats::Metric::Lines;
}
