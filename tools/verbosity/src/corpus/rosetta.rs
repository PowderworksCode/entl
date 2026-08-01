//! Reads a local checkout of the Rosetta Code Data project.
//!
//! Layout: `Task/<task>/<Language>/<solution>.<extension>`. A task usually has
//! several solutions per language, so each (language, task) pair is reduced to
//! the median solution before any language is compared to another.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use entl_codebase::comment_syntax;

use super::{Corpus, Samples};
use crate::measure::{Measurement, measure};

/// Maps an Entl language profile onto the corpus. The directory names and the
/// one extension the corpus actually uses for each are Rosetta Code's business,
/// not Entl's, so they live here rather than in a language profile.
pub struct Mapping {
    pub language: &'static str,
    pub directory: &'static str,
    pub extension: &'static str,
}

pub const MAPPINGS: &[Mapping] = &[
    Mapping {
        language: "c",
        directory: "C",
        extension: "c",
    },
    Mapping {
        language: "c-sharp",
        directory: "C-sharp",
        extension: "cs",
    },
    Mapping {
        language: "cpp",
        directory: "C++",
        extension: "cpp",
    },
    Mapping {
        language: "go",
        directory: "Go",
        extension: "go",
    },
    Mapping {
        language: "java",
        directory: "Java",
        extension: "java",
    },
    Mapping {
        language: "javascript",
        directory: "JavaScript",
        extension: "js",
    },
    // The corpus files Kotlin solutions as scripts, which skip the class and
    // `main` ceremony a Kotlin application file would carry.
    Mapping {
        language: "kotlin",
        directory: "Kotlin",
        extension: "kts",
    },
    Mapping {
        language: "make",
        directory: "Make",
        extension: "make",
    },
    Mapping {
        language: "php",
        directory: "PHP",
        extension: "php",
    },
    Mapping {
        language: "python",
        directory: "Python",
        extension: "py",
    },
    Mapping {
        language: "ruby",
        directory: "Ruby",
        extension: "rb",
    },
    Mapping {
        language: "rust",
        directory: "Rust",
        extension: "rs",
    },
    Mapping {
        language: "scala",
        directory: "Scala",
        extension: "scala",
    },
    Mapping {
        language: "shell",
        directory: "UNIX-Shell",
        extension: "sh",
    },
    Mapping {
        language: "sql",
        directory: "SQL",
        extension: "sql",
    },
    Mapping {
        language: "swift",
        directory: "Swift",
        extension: "swift",
    },
    Mapping {
        language: "typescript",
        directory: "TypeScript",
        extension: "ts",
    },
    Mapping {
        language: "zig",
        directory: "Zig",
        extension: "zig",
    },
];

pub fn read(root: &Path) -> Result<Corpus, String> {
    let tasks_root = root.join("Task");
    if !tasks_root.is_dir() {
        return Err(format!(
            "{} does not look like a RosettaCodeData checkout: no Task/ directory",
            root.display()
        ));
    }

    let mut samples: BTreeMap<&'static str, Samples> = BTreeMap::new();
    let mut skipped = Vec::new();
    let mut tasks = 0;

    let mut entries = fs::read_dir(&tasks_root)
        .map_err(|error| format!("read {}: {error}", tasks_root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    entries.sort();

    for task_directory in entries {
        let Some(task) = task_directory.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        tasks += 1;
        for mapping in MAPPINGS {
            let Some(syntax) = comment_syntax(mapping.language) else {
                skipped.push(format!(
                    "{}: no comment syntax in the language profile",
                    mapping.language
                ));
                continue;
            };
            let solutions = task_directory.join(mapping.directory);
            let Ok(files) = fs::read_dir(&solutions) else {
                continue;
            };
            let mut measurements = Vec::new();
            for file in files.filter_map(Result::ok) {
                let path = file.path();
                if path.extension().and_then(|value| value.to_str()) != Some(mapping.extension) {
                    continue;
                }
                let Ok(bytes) = fs::read(&path) else {
                    continue;
                };
                let measured = measure(&String::from_utf8_lossy(&bytes), syntax);
                if !measured.is_empty() {
                    measurements.push(measured);
                }
            }
            if let Some(median) = median(&mut measurements) {
                samples
                    .entry(mapping.language)
                    .or_default()
                    .insert(task.to_owned(), median);
            }
        }
    }

    Ok(Corpus {
        revision: super::revision(root),
        units: tasks,
        samples,
        skipped,
    })
}

/// Reduces a task's solutions to one measurement. The median resists both the
/// code-golf entry and the heavily annotated tutorial entry that many popular
/// tasks accumulate.
fn median(measurements: &mut [Measurement]) -> Option<Measurement> {
    if measurements.is_empty() {
        return None;
    }
    Some(Measurement {
        lines: median_of(measurements, |measurement| measurement.lines),
        bytes: median_of(measurements, |measurement| measurement.bytes),
    })
}

fn median_of(measurements: &[Measurement], field: fn(&Measurement) -> u32) -> u32 {
    let mut values = measurements.iter().map(field).collect::<Vec<_>>();
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        values[middle - 1].midpoint(values[middle])
    }
}
