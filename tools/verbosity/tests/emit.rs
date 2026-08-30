// Tests for `src/emit.rs`: the table a run prints.
//
// The table is the whole output, so what matters is that it renders from a
// report with nothing in it. An empty corpus should print an empty table rather
// than panic on the first index.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::collections::BTreeMap;

use verbosity::corpus::Source;
use verbosity::emit::{Report, table};

#[test]
fn an_empty_report_still_renders() {
    let samples = BTreeMap::new();
    let empty = BTreeMap::new();
    let report = Report {
        source: Source::Exercism,
        revision: "unknown",
        tasks: 0,
        baseline: "rust",
        samples: &samples,
        pairs: &[],
        bytes: &empty,
        lines: &empty,
        core: &[],
        panel: 0,
        balanced: &empty,
        minimum_shared_tasks: 1,
        minimum_language_tasks: 1,
    };
    assert!(!table(&report).is_empty());
}
