//! Reads a checkout of [mal](https://github.com/kanaka/mal), one Lisp
//! interpreter implemented in each of 80-odd languages.
//!
//! This corpus answers a question the other two cannot. Rosetta Code and
//! Exercism are both collections of small self-contained pieces; a mal
//! implementation is a working interpreter of a thousand to five thousand lines,
//! and every implementation follows the same eleven-step guide and passes the
//! same test suite. It is the only mid-sized program available in all of Entl's
//! languages with its scope pinned by something other than good intentions.
//!
//! The cost is that there is exactly **one unit**: the whole implementation.
//! Finer units were tried and rejected, because implementations factor
//! differently — C++ has an `Environment` and no `Printer`, Zig ships its own
//! hash map and linked list, and matching files by name would credit those to
//! nobody. Whole implementations are the only comparison where every byte is
//! counted exactly once for every language.
//!
//! With one shared unit the ratios are exactly transitive and every fitted
//! deviation is zero. That is arithmetic, not agreement: it means the index is
//! reproducing sixteen measurements rather than reconciling many. There is no
//! spread here to put an error bar on, and one program is one genre — an
//! interpreter leans on tagged unions, recursion, and manual memory for a
//! garbage-collected target. Read it as a single mid-sized data point.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use entl_codebase::comment_syntax;

use super::{Corpus, Samples};
use crate::measure::{Measurement, measure};

/// The unit key. There is one comparison here, so it is named rather than
/// enumerated.
const UNIT: &str = "interpreter";

/// Maps an Entl language profile onto an implementation directory and the
/// extensions that implementation is written in.
///
/// Extensions are listed per track rather than taken from the language profile
/// because `.h` belongs to Entl's `c` profile while C++ implementations use it
/// too, and because several tracks have more than one implementation to choose
/// between (`swift3`, `swift4`, `swift6`) where only the current one belongs in
/// a comparison.
pub struct Implementation {
    pub language: &'static str,
    pub directory: &'static str,
    pub extensions: &'static [&'static str],
}

pub const IMPLEMENTATIONS: &[Implementation] = &[
    Implementation {
        language: "c",
        directory: "c",
        extensions: &["c", "h"],
    },
    Implementation {
        language: "c-sharp",
        directory: "cs",
        extensions: &["cs"],
    },
    Implementation {
        language: "cpp",
        directory: "cpp",
        extensions: &["cpp", "h", "hpp"],
    },
    Implementation {
        language: "go",
        directory: "go",
        extensions: &["go"],
    },
    Implementation {
        language: "java",
        directory: "java",
        extensions: &["java"],
    },
    Implementation {
        language: "javascript",
        directory: "js",
        extensions: &["js"],
    },
    Implementation {
        language: "kotlin",
        directory: "kotlin",
        extensions: &["kt"],
    },
    Implementation {
        language: "php",
        directory: "php",
        extensions: &["php"],
    },
    Implementation {
        language: "python",
        directory: "python3",
        extensions: &["py"],
    },
    Implementation {
        language: "ruby",
        directory: "ruby",
        extensions: &["rb"],
    },
    Implementation {
        language: "rust",
        directory: "rust",
        extensions: &["rs"],
    },
    Implementation {
        language: "scala",
        directory: "scala",
        extensions: &["scala"],
    },
    Implementation {
        language: "shell",
        directory: "bash",
        extensions: &["sh"],
    },
    Implementation {
        language: "swift",
        directory: "swift6",
        extensions: &["swift"],
    },
    Implementation {
        language: "typescript",
        directory: "ts",
        extensions: &["ts"],
    },
    Implementation {
        language: "zig",
        directory: "zig",
        extensions: &["zig"],
    },
];

pub fn looks_like_implementations(root: &Path) -> bool {
    root.join("impls").is_dir() && root.join("impls").join("rust").is_dir()
}

pub fn read(root: &Path) -> Result<Corpus, String> {
    if !looks_like_implementations(root) {
        return Err(format!(
            "{} does not look like a mal checkout: no impls/ directory",
            root.display()
        ));
    }

    let mut samples: BTreeMap<&'static str, Samples> = BTreeMap::new();
    let mut skipped = Vec::new();

    for implementation in IMPLEMENTATIONS {
        let root = root.join("impls").join(implementation.directory);
        if !root.is_dir() {
            skipped.push(format!(
                "{}: no implementation at impls/{}",
                implementation.language, implementation.directory
            ));
            continue;
        }
        let Some(syntax) = comment_syntax(implementation.language) else {
            skipped.push(format!(
                "{}: no comment syntax in the language profile",
                implementation.language
            ));
            continue;
        };

        let mut total = Measurement { lines: 0, bytes: 0 };
        let mut files = Vec::new();
        collect(&root, implementation.extensions, &mut files);
        files.sort();
        for path in files {
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let measured = measure(&String::from_utf8_lossy(&bytes), syntax);
            total.lines += measured.lines;
            total.bytes += measured.bytes;
        }
        if total.is_empty() {
            skipped.push(format!(
                "{}: no source found under impls/{}",
                implementation.language, implementation.directory
            ));
            continue;
        }
        samples
            .entry(implementation.language)
            .or_default()
            .insert(UNIT.to_owned(), total);
    }

    Ok(Corpus {
        revision: super::revision(root),
        units: 1,
        samples,
        skipped,
    })
}

fn collect(directory: &Path, extensions: &[&str], found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if !is_excluded_directory(name) {
                collect(&path, extensions, found);
            }
            continue;
        }
        let matches = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension));
        if matches && !is_excluded_file(name) {
            found.push(path);
        }
    }
}

fn is_excluded_directory(name: &str) -> bool {
    // Dependencies and build output are not the implementation, and `tests`
    // holds the shared suite rather than anyone's answer to it.
    matches!(
        name,
        "tests" | "test" | "node_modules" | "target" | "build" | ".git" | "vendor" | "obj" | "bin"
    )
}

/// Excludes the test files some implementations keep beside their source, and
/// the build scripts that happen to be written in the implementation's own
/// language. Left in, `python3/test_step4.py` would be charged to Python as
/// interpreter code, and `zig/build.zig` to Zig.
fn is_excluded_file(name: &str) -> bool {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    stem.starts_with("test_")
        || stem.ends_with("_test")
        || stem.ends_with("Test")
        || stem.eq_ignore_ascii_case("build")
        || stem.eq_ignore_ascii_case("package")
        || stem.eq_ignore_ascii_case("conftest")
}
