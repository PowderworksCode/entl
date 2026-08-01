//! Turns per-task measurements into pairwise ratios and a single index.
//!
//! Two languages are compared only on tasks that both implement, so every
//! ratio is a paired measurement. Because each pair is averaged over a
//! different set of tasks, the ratios are not exactly transitive: C/Java need
//! not equal (C/Lisp)(Lisp/Java). The single index is the least-squares fit
//! that reconciles them, and the residuals report how much reconciling it took.

use std::collections::{BTreeMap, BTreeSet};

use crate::corpus::Samples;
use crate::measure::Measurement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Bytes,
    Lines,
}

impl Metric {
    pub fn of(&self, measurement: &Measurement) -> f64 {
        match self {
            Metric::Bytes => f64::from(measurement.bytes),
            Metric::Lines => f64::from(measurement.lines),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub left: &'static str,
    pub right: &'static str,
    /// Geometric mean of left/right over the shared tasks, per metric.
    pub bytes: f64,
    pub lines: f64,
    pub tasks: u32,
}

/// Computes every pair whose languages share at least `minimum` tasks. Pairs
/// are emitted once, with `left < right`; the reciprocal is exact by
/// construction rather than by a second, separately rounded average.
pub fn pairs(samples: &BTreeMap<&'static str, Samples>, minimum: u32) -> Vec<Pair> {
    let languages = samples.keys().copied().collect::<Vec<_>>();
    let mut pairs = Vec::new();
    for (index, left) in languages.iter().enumerate() {
        for right in &languages[index + 1..] {
            let shared = shared_tasks(&samples[left], &samples[right]);
            if shared.len() < minimum as usize {
                continue;
            }
            pairs.push(Pair {
                left,
                right,
                bytes: geometric_ratio(&shared, Metric::Bytes),
                lines: geometric_ratio(&shared, Metric::Lines),
                tasks: shared.len() as u32,
            });
        }
    }
    pairs
}

fn shared_tasks(left: &Samples, right: &Samples) -> Vec<(Measurement, Measurement)> {
    left.iter()
        .filter_map(|(task, measurement)| right.get(task).map(|other| (*measurement, *other)))
        .filter(|(left, right)| !left.is_empty() && !right.is_empty())
        .collect()
}

fn geometric_ratio(shared: &[(Measurement, Measurement)], metric: Metric) -> f64 {
    let total: f64 = shared
        .iter()
        .map(|(left, right)| (metric.of(left) / metric.of(right)).ln())
        .sum();
    (total / shared.len() as f64).exp()
}

/// The least-squares verbosity index: the per-language value whose differences
/// best explain every measured pair ratio, weighted by shared task count and
/// normalized so `baseline` is 1.0.
pub fn fit(pairs: &[Pair], metric: Metric, baseline: &str) -> BTreeMap<&'static str, f64> {
    let languages = languages_in(pairs);
    let count = languages.len();
    let position = languages
        .iter()
        .enumerate()
        .map(|(index, language)| (*language, index))
        .collect::<BTreeMap<_, _>>();

    // Weighted graph Laplacian: L v = r, where the gauge freedom (v is only
    // determined up to a constant) is fixed by pinning the baseline to zero.
    let mut matrix = vec![vec![0.0f64; count]; count];
    let mut target = vec![0.0f64; count];
    for pair in pairs {
        let (left, right) = (position[pair.left], position[pair.right]);
        let weight = f64::from(pair.tasks);
        let observed = ratio(pair, metric).ln();
        matrix[left][left] += weight;
        matrix[right][right] += weight;
        matrix[left][right] -= weight;
        matrix[right][left] -= weight;
        target[left] += weight * observed;
        target[right] -= weight * observed;
    }

    let anchor = position[baseline];
    matrix[anchor].fill(0.0);
    for row in matrix.iter_mut() {
        row[anchor] = 0.0;
    }
    matrix[anchor][anchor] = 1.0;
    target[anchor] = 0.0;

    let solution = solve(matrix, target);
    languages
        .iter()
        .enumerate()
        .map(|(index, language)| (*language, solution[index].exp()))
        .collect()
}

pub fn ratio(pair: &Pair, metric: Metric) -> f64 {
    match metric {
        Metric::Bytes => pair.bytes,
        Metric::Lines => pair.lines,
    }
}

/// How far a measured pair ratio sits from the fitted index, in natural log
/// units. Zero means the single index explains that pair exactly.
pub fn residual(pair: &Pair, metric: Metric, index: &BTreeMap<&'static str, f64>) -> f64 {
    ratio(pair, metric).ln() - (index[pair.left] / index[pair.right]).ln()
}

/// The tasks every language in `languages` implements. On this balanced panel
/// the ratios are transitive by construction, so comparing it to the all-pairs
/// fit isolates how much of the disagreement comes from each pair being
/// averaged over a different slice of the corpus.
pub fn balanced_panel(
    samples: &BTreeMap<&'static str, Samples>,
    languages: &[&'static str],
) -> BTreeSet<String> {
    let mut panel: Option<BTreeSet<String>> = None;
    for language in languages {
        let Some(tasks) = samples.get(language) else {
            return BTreeSet::new();
        };
        let owned = tasks
            .iter()
            .filter(|(_, measurement)| !measurement.is_empty())
            .map(|(task, _)| task.clone())
            .collect::<BTreeSet<_>>();
        panel = Some(match panel {
            None => owned,
            Some(existing) => existing.intersection(&owned).cloned().collect(),
        });
    }
    panel.unwrap_or_default()
}

pub fn balanced_index(
    samples: &BTreeMap<&'static str, Samples>,
    languages: &[&'static str],
    panel: &BTreeSet<String>,
    metric: Metric,
    baseline: &str,
) -> BTreeMap<&'static str, f64> {
    let mean = |language: &&'static str| -> f64 {
        let tasks = &samples[*language];
        let total: f64 = panel.iter().map(|task| metric.of(&tasks[task]).ln()).sum();
        total / panel.len() as f64
    };
    let anchor = languages
        .iter()
        .find(|language| **language == baseline)
        .map(&mean)
        .unwrap_or_default();
    languages
        .iter()
        .map(|language| (*language, (mean(language) - anchor).exp()))
        .collect()
}

/// The worst triangle in the pair table: the three languages where composing
/// two ratios least agrees with the third, measured directly.
#[derive(Debug, Clone)]
pub struct Triangle {
    pub through: [&'static str; 3],
    pub direct: f64,
    pub composed: f64,
    pub tasks: [u32; 3],
}

pub fn triangles(pairs: &[Pair], metric: Metric) -> Vec<Triangle> {
    let lookup = pairs
        .iter()
        .map(|pair| ((pair.left, pair.right), pair))
        .collect::<BTreeMap<_, _>>();
    let languages = languages_in(pairs);
    let mut triangles = Vec::new();
    for (first, left) in languages.iter().enumerate() {
        for (second, middle) in languages.iter().enumerate().skip(first + 1) {
            for right in languages.iter().skip(second + 1) {
                let (Some(a), Some(b), Some(c)) = (
                    lookup.get(&(*left, *middle)),
                    lookup.get(&(*middle, *right)),
                    lookup.get(&(*left, *right)),
                ) else {
                    continue;
                };
                triangles.push(Triangle {
                    through: [*left, *middle, *right],
                    direct: ratio(c, metric),
                    composed: ratio(a, metric) * ratio(b, metric),
                    tasks: [a.tasks, b.tasks, c.tasks],
                });
            }
        }
    }
    triangles.sort_by(|left, right| {
        disagreement(right)
            .partial_cmp(&disagreement(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    triangles
}

pub fn disagreement(triangle: &Triangle) -> f64 {
    (triangle.composed / triangle.direct).ln().abs()
}

pub fn languages_in(pairs: &[Pair]) -> Vec<&'static str> {
    let mut languages = pairs
        .iter()
        .flat_map(|pair| [pair.left, pair.right])
        .collect::<Vec<_>>();
    languages.sort_unstable();
    languages.dedup();
    languages
}

/// Gaussian elimination with partial pivoting. The system is one row per
/// language, so it stays small enough that a dense solve is the simple choice.
fn solve(mut matrix: Vec<Vec<f64>>, mut target: Vec<f64>) -> Vec<f64> {
    let count = target.len();
    for column in 0..count {
        let pivot = (column..count)
            .max_by(|left, right| {
                matrix[*left][column]
                    .abs()
                    .partial_cmp(&matrix[*right][column].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(column);
        matrix.swap(column, pivot);
        target.swap(column, pivot);
        let divisor = matrix[column][column];
        if divisor.abs() < f64::EPSILON {
            continue;
        }
        let (eliminated, remaining) = matrix.split_at_mut(column + 1);
        let pivot_row = &eliminated[column];
        for (offset, row) in remaining.iter_mut().enumerate() {
            let factor = row[column] / divisor;
            if factor == 0.0 {
                continue;
            }
            for (cell, pivot_cell) in row.iter_mut().zip(pivot_row).skip(column) {
                *cell -= factor * pivot_cell;
            }
            target[column + 1 + offset] -= factor * target[column];
        }
    }

    let mut solution = vec![0.0; count];
    for row in (0..count).rev() {
        let mut value = target[row];
        for column in row + 1..count {
            value -= matrix[row][column] * solution[column];
        }
        let divisor = matrix[row][row];
        solution[row] = if divisor.abs() < f64::EPSILON {
            0.0
        } else {
            value / divisor
        };
    }
    solution
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
