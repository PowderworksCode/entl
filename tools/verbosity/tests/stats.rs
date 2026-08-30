// Tests for `src/stats.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::collections::BTreeMap;

use verbosity::corpus::Samples;
use verbosity::measure::Measurement;
use verbosity::stats::*;

fn samples(entries: &[(&'static str, &[(&str, u32)])]) -> BTreeMap<&'static str, Samples> {
    entries
        .iter()
        .map(|(language, tasks)| {
            let tasks = tasks
                .iter()
                .map(|(task, bytes)| {
                    (
                        (*task).to_owned(),
                        Measurement {
                            lines: *bytes,
                            bytes: *bytes,
                        },
                    )
                })
                .collect::<Samples>();
            (*language, tasks)
        })
        .collect()
}

#[test]
fn ratio_is_the_geometric_mean_over_shared_tasks() {
    let samples = samples(&[
        ("a", &[("one", 10), ("two", 40), ("three", 5)]),
        ("b", &[("one", 5), ("two", 10)]),
    ]);
    let pairs = pairs(&samples, 1);
    assert_eq!(pairs.len(), 1);
    // Shared tasks only: 10/5 and 40/10, geometric mean sqrt(2 * 4).
    assert!((pairs[0].bytes - 8.0f64.sqrt()).abs() < 1e-12);
    assert_eq!(pairs[0].tasks, 2);
}

#[test]
fn fit_recovers_an_exactly_transitive_system() {
    let samples = samples(&[
        ("a", &[("one", 100), ("two", 200)]),
        ("b", &[("one", 50), ("two", 100)]),
        ("c", &[("one", 25), ("two", 50)]),
    ]);
    let index = fit(&pairs(&samples, 1), Metric::Bytes, "c");
    assert!((index["c"] - 1.0).abs() < 1e-12);
    assert!((index["b"] - 2.0).abs() < 1e-12);
    assert!((index["a"] - 4.0).abs() < 1e-12);
}

#[test]
fn balanced_panel_is_the_task_intersection() {
    let samples = samples(&[
        ("a", &[("one", 1), ("two", 1)]),
        ("b", &[("two", 1), ("three", 1)]),
    ]);
    let panel = balanced_panel(&samples, &["a", "b"]);
    assert_eq!(
        panel.into_iter().collect::<Vec<_>>(),
        vec!["two".to_owned()]
    );
}

#[test]
fn residuals_vanish_when_one_index_explains_every_pair() {
    let samples = samples(&[
        ("a", &[("one", 100), ("two", 200)]),
        ("b", &[("one", 50), ("two", 100)]),
    ]);
    let pairs = pairs(&samples, 1);
    let index = fit(&pairs, Metric::Bytes, "b");
    assert!(residual(&pairs[0], Metric::Bytes, &index).abs() < 1e-12);
}
