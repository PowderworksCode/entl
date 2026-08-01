//! Measures how much source text each language needs for the same task.
//!
//! Reads a local checkout of the Rosetta Code Data project, compares Entl's
//! language profiles on the tasks they both implement, and regenerates
//! `crates/entl-codebase/src/profiles/verbosity.rs` and `docs/verbosity-*.md`.
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

use corpus::Source;
use stats::Metric;

/// A language covering this fraction of the corpus joins the balanced panel,
/// where every ratio comes from an identical set of units. Expressed as a
/// fraction because the corpora differ in size by an order of magnitude.
const CORE_COVERAGE: f64 = 1.0 / 3.0;

struct Options {
    corpus: PathBuf,
    source: Option<Source>,
    root: PathBuf,
    baseline: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("verbosity: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse()?;
    let source = match options.source {
        Some(source) => source,
        None => Source::detect(&options.corpus).ok_or_else(|| {
            format!(
                "cannot tell what {} is; pass --source rosetta or --source exercism",
                options.corpus.display()
            )
        })?,
    };
    eprintln!("reading {} as {}", options.corpus.display(), source.label());
    let corpus = corpus::read(source, &options.corpus)?;
    for note in &corpus.skipped {
        eprintln!("skipped {note}");
    }

    let (minimum_shared, minimum_language) = source.floors();
    let samples = corpus
        .samples
        .into_iter()
        .filter(|(_, tasks)| tasks.len() >= minimum_language)
        .collect::<BTreeMap<_, _>>();
    if !samples.contains_key(options.baseline.as_str()) {
        return Err(format!(
            "baseline {} has fewer than {minimum_language} units in the corpus",
            options.baseline
        ));
    }
    eprintln!(
        "{} {}s, {} languages above the {minimum_language}-{} floor",
        corpus.units,
        source.unit(),
        samples.len(),
        source.unit()
    );

    let pairs = stats::pairs(&samples, minimum_shared);
    if pairs.is_empty() {
        return Err("no language pair shares enough tasks to compare".to_owned());
    }
    let measured = stats::languages_in(&pairs);
    let bytes = stats::fit(&pairs, Metric::Bytes, &options.baseline);
    let lines = stats::fit(&pairs, Metric::Lines, &options.baseline);

    let core_floor = (corpus.units as f64 * CORE_COVERAGE).ceil() as usize;
    let mut core = measured
        .iter()
        .copied()
        .filter(|language| samples[language].len() >= core_floor)
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
        "balanced panel: {} {}s across {} languages (floor {core_floor})",
        panel.len(),
        source.unit(),
        core.len()
    );

    let report = emit::Report {
        source,
        revision: &corpus.revision,
        tasks: corpus.units,
        baseline: &options.baseline,
        samples: &samples,
        pairs: &pairs,
        bytes: &bytes,
        lines: &lines,
        core: &core,
        panel: panel.len(),
        balanced: &balanced,
        minimum_shared_tasks: minimum_shared,
        minimum_language_tasks: minimum_language,
    };

    let table = options
        .root
        .join("crates/entl-codebase/src/profiles/verbosity.rs");
    let document = options
        .root
        .join(format!("docs/verbosity-{}.md", source.id()));
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
    let mut source = None;
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
            "--source" => source = Some(Source::parse(&value()?)?),
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
        source,
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
usage: verbosity --corpus <checkout> [--source rosetta|exercism]
                        [--root <entl repo>] [--baseline <language>]

The source is detected from the layout when it can be. Rosetta Code is one
checkout; Exercism is a directory of per-track checkouts.

  git clone https://github.com/acmeism/RosettaCodeData
  cargo run --manifest-path tools/verbosity/Cargo.toml -- --corpus ../RosettaCodeData

  mkdir exercism && cd exercism
  for t in c cpp csharp go java javascript kotlin php python ruby rust scala \\
           swift typescript zig bash; do git clone --depth 1 \\
    https://github.com/exercism/$t $t; done";
