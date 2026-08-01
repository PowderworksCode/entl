//! The corpora a verbosity measurement can be taken from.
//!
//! Both corpora answer the same question — how much source does this language
//! need for a task another language also implements — and they fail in
//! different ways, which is the reason to keep both. Rosetta Code is large and
//! uncontrolled: solutions are contributed, and two entries for one task do not
//! always answer the same question. Exercism is small and controlled: every
//! exercise has one reference solution written by track maintainers against a
//! shared specification and a shared test suite.
//!
//! An index the two agree on is a fact about the languages. One they disagree
//! on is a fact about the corpora.

pub mod exercism;
pub mod mal;
pub mod rosetta;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::measure::Measurement;

/// The median measurement of every task a language implements, keyed by task.
pub type Samples = BTreeMap<String, Measurement>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Rosetta,
    Exercism,
    Mal,
}

impl Source {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "rosetta" => Ok(Self::Rosetta),
            "exercism" => Ok(Self::Exercism),
            "mal" => Ok(Self::Mal),
            other => Err(format!(
                "unknown source {other:?}, expected rosetta, exercism, or mal"
            )),
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Rosetta => "rosetta",
            Self::Exercism => "exercism",
            Self::Mal => "mal",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Rosetta => "Rosetta Code",
            Self::Exercism => "Exercism",
            Self::Mal => "mal",
        }
    }

    /// What one unit of comparison is called in this corpus.
    pub fn unit(&self) -> &'static str {
        match self {
            Self::Rosetta => "task",
            Self::Exercism => "exercise",
            Self::Mal => "implementation",
        }
    }

    pub fn home(&self) -> &'static str {
        match self {
            Self::Rosetta => "https://github.com/acmeism/RosettaCodeData",
            Self::Exercism => "https://github.com/exercism",
            Self::Mal => "https://github.com/kanaka/mal",
        }
    }

    /// The license the corpus content carries. Neither corpus is
    /// redistributed; this records what Entl is measuring, not shipping.
    pub fn license(&self) -> &'static str {
        match self {
            Self::Rosetta => "GNU Free Documentation License 1.2",
            Self::Exercism => "MIT License",
            Self::Mal => "Mozilla Public License 2.0",
        }
    }

    /// What kind of program the corpus is made of, which is what the numbers
    /// can and cannot be carried to.
    pub fn character(&self) -> &'static str {
        match self {
            Self::Rosetta | Self::Exercism => {
                "Verbosity here is source size on small self-contained pieces, \
                 which rewards languages with terse standard libraries and charges \
                 languages that require declarations, imports, or a class to hold \
                 `main`. It does not extrapolate to large programs: measured on a \
                 mid-sized one instead, the same languages spread roughly twice as \
                 far apart."
            }
            Self::Mal => {
                "Verbosity here is source size on one mid-sized program — a Lisp \
                 interpreter of one to five thousand lines — so it reaches a scale \
                 the collection corpora cannot. One program is also one genre, and \
                 this genre leans on tagged unions, recursion, and manual memory \
                 for a garbage-collected target."
            }
        }
    }

    /// How one comparable measurement per (language, unit) is arrived at.
    /// This is the step where the two corpora differ most.
    pub fn selection(&self) -> &'static str {
        match self {
            Self::Mal => {
                "Each language contributes exactly one unit: its **whole \
                 implementation**, every source file under `impls/<lang>` \
                 except tests and build scripts. Finer units were tried and \
                 rejected — implementations factor differently, and matching \
                 files by name credits nobody for the hash map Zig had to write \
                 itself. Whole implementations count every byte exactly once."
            }
            Self::Rosetta => {
                "A task usually has several contributed solutions per language. \
                 Each (language, task) pair is reduced to the **median** solution, \
                 which resists both the code-golf entry and the annotated tutorial \
                 entry that popular tasks accumulate."
            }
            Self::Exercism => {
                "Each exercise has exactly one **reference solution** per track, \
                 named by `files.example` in the exercise's `.meta/config.json` and \
                 verified against the track's test suite. Nothing needs averaging: \
                 the corpus already holds one canonical answer per (language, \
                 exercise). Where a solution spans several files, they are summed."
            }
        }
    }

    /// The fewest shared units a pair needs before its ratio is reported, and
    /// the fewest units a language needs to be included at all.
    ///
    /// mal is one program, so its only honest floor is one. The other corpora
    /// are collections, where a handful of shared units is noise.
    pub fn floors(&self) -> (u32, usize) {
        match self {
            Self::Rosetta | Self::Exercism => (25, 50),
            Self::Mal => (1, 1),
        }
    }

    /// Whether ratios in this corpus carry a distribution at all. With a single
    /// shared unit every pair is one measurement, the fit is exact, and the
    /// spread statistics downstream would be reporting arithmetic as agreement.
    pub fn has_spread(&self) -> bool {
        !matches!(self, Self::Mal)
    }

    /// Recognizes a checkout by its layout, so `--source` can usually be
    /// omitted.
    pub fn detect(root: &Path) -> Option<Self> {
        if root.join("Task").is_dir() {
            Some(Self::Rosetta)
        } else if mal::looks_like_implementations(root) {
            Some(Self::Mal)
        } else if exercism::looks_like_tracks(root) {
            Some(Self::Exercism)
        } else {
            None
        }
    }
}

pub struct Corpus {
    pub revision: String,
    /// Units available in the corpus, whether or not any language implements
    /// them.
    pub units: usize,
    pub samples: BTreeMap<&'static str, Samples>,
    pub skipped: Vec<String>,
}

pub fn read(source: Source, root: &Path) -> Result<Corpus, String> {
    let mut corpus = match source {
        Source::Rosetta => rosetta::read(root),
        Source::Exercism => exercism::read(root),
        Source::Mal => mal::read(root),
    }?;
    corpus.skipped.sort();
    corpus.skipped.dedup();
    Ok(corpus)
}

/// The revision of a checkout, for provenance. Returns `unknown` rather than
/// failing: a corpus copied without its history is still measurable.
pub fn revision(root: &Path) -> String {
    Command::new("git")
        .args(["-C", &root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}
