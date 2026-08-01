//! Measures how much source text each language needs for the same task.
//!
//! Reads a local checkout of the Rosetta Code Data project, compares Entl's
//! language profiles on the tasks they both implement, and regenerates
//! `crates/entl-codebase/src/profiles/verbosity.rs` and `docs/verbosity.md`.
//!
//! Only derived statistics are written out. Rosetta Code's content is licensed
//! under the GNU Free Documentation License 1.2, which Entl's MIT license
//! cannot absorb, so no corpus source ever lands in the repository.

mod corpus;
mod emit;
mod measure;
mod stats;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use stats::Metric;

/// Below this many shared tasks a pair ratio is noise, not a measurement.
const MINIMUM_SHARED_TASKS: u32 = 25;
/// Below this many tasks overall a language cannot anchor a useful comparison.
const MINIMUM_LANGUAGE_TASKS: usize = 50;
/// Languages this well represented define the balanced panel, where every
/// ratio comes from an identical set of tasks.
const CORE_LANGUAGE_TASKS: usize = 600;

struct Options {
    corpus: PathBuf,
    root: PathBuf,
    baseline: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rosetta-verbosity: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse()?;
    eprintln!("reading {}", options.corpus.display());
    let corpus = corpus::read(&options.corpus)?;
    for note in &corpus.skipped {
        eprintln!("skipped {note}");
    }

    let samples = corpus
        .samples
        .into_iter()
        .filter(|(_, tasks)| tasks.len() >= MINIMUM_LANGUAGE_TASKS)
        .collect::<BTreeMap<_, _>>();
    if !samples.contains_key(options.baseline.as_str()) {
        return Err(format!(
            "baseline {} has fewer than {MINIMUM_LANGUAGE_TASKS} tasks in the corpus",
            options.baseline
        ));
    }
    eprintln!(
        "{} tasks, {} languages above the {MINIMUM_LANGUAGE_TASKS}-task floor",
        corpus.tasks,
        samples.len()
    );

    let pairs = stats::pairs(&samples, MINIMUM_SHARED_TASKS);
    if pairs.is_empty() {
        return Err("no language pair shares enough tasks to compare".to_owned());
    }
    let measured = stats::languages_in(&pairs);
    let bytes = stats::fit(&pairs, Metric::Bytes, &options.baseline);
    let lines = stats::fit(&pairs, Metric::Lines, &options.baseline);

    let mut core = measured
        .iter()
        .copied()
        .filter(|language| samples[language].len() >= CORE_LANGUAGE_TASKS)
        .collect::<Vec<_>>();
    if !core.contains(&options.baseline.as_str()) {
        core.push(
            measured
                .iter()
                .find(|language| **language == options.baseline)
                .copied()
                .ok_or("baseline dropped out of the pair table")?,
        );
        core.sort_unstable();
    }
    let panel = stats::balanced_panel(&samples, &core);
    let balanced = stats::balanced_index(&samples, &core, &panel, Metric::Bytes, &options.baseline);
    eprintln!(
        "balanced panel: {} tasks across {} languages",
        panel.len(),
        core.len()
    );

    let report = emit::Report {
        revision: &corpus.revision,
        tasks: corpus.tasks,
        baseline: &options.baseline,
        samples: &samples,
        pairs: &pairs,
        bytes: &bytes,
        lines: &lines,
        core: &core,
        panel: panel.len(),
        balanced: &balanced,
        minimum_shared_tasks: MINIMUM_SHARED_TASKS,
        minimum_language_tasks: MINIMUM_LANGUAGE_TASKS,
    };

    let table = options
        .root
        .join("crates/entl-codebase/src/profiles/verbosity.rs");
    let document = options.root.join("docs/verbosity.md");
    write(&table, &emit::table(&report))?;
    write(&document, &emit::document(&report))?;
    format(&table)?;
    eprintln!("wrote {}", table.display());
    eprintln!("wrote {}", document.display());
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|error| format!("write {}: {error}", path.display()))
}

/// Hands the generated table to rustfmt so `cargo fmt --check` stays clean
/// without this tool having to reproduce rustfmt's layout rules.
fn format(path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(path)
        .output()
        .map_err(|error| format!("run rustfmt: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustfmt failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn parse() -> Result<Options, String> {
    let mut corpus = None;
    let mut root = None;
    let mut baseline = "c".to_owned();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--corpus" => corpus = Some(PathBuf::from(value()?)),
            "--root" => root = Some(PathBuf::from(value()?)),
            "--baseline" => baseline = value()?,
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unexpected argument {other}\n\n{USAGE}")),
        }
    }
    Ok(Options {
        corpus: corpus.ok_or_else(|| format!("--corpus is required\n\n{USAGE}"))?,
        root: root.unwrap_or_else(default_root),
        baseline,
    })
}

fn default_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

const USAGE: &str = "\
usage: rosetta-verbosity --corpus <RosettaCodeData checkout> [--root <entl repo>] [--baseline <language>]

  git clone https://github.com/acmeism/RosettaCodeData
  cargo run --manifest-path tools/rosetta-verbosity/Cargo.toml -- --corpus ../RosettaCodeData";
