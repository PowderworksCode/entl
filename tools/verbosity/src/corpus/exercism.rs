//! Reads a set of [Exercism](https://github.com/exercism) track checkouts.
//!
//! Layout: `<track>/exercises/practice/<slug>/.meta/config.json`, whose
//! `files.example` names the track's reference solution. That pointer is the
//! reason this corpus is worth reading: there is exactly one canonical solution
//! per (language, exercise), written by track maintainers against a shared
//! specification and verified by a shared test suite, so no averaging over
//! contributed entries is needed to get a comparable number.
//!
//! Only *practice* exercises are read. Concept exercises teach a track's own
//! syllabus and have no counterpart in another language.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use entl::codebase::{LanguageRole, comment_syntax, language_profile_for_extension};

use super::{Corpus, Samples};
use crate::measure::{Measurement, measure};

/// Maps an Entl language profile onto a track directory. Only the names that
/// differ need saying, but listing all of them keeps the corpus's membership
/// explicit rather than implied by whatever happens to be on disk.
pub const TRACKS: &[(&str, &str)] = &[
    ("c", "c"),
    ("c-sharp", "csharp"),
    ("cpp", "cpp"),
    ("go", "go"),
    ("java", "java"),
    ("javascript", "javascript"),
    ("kotlin", "kotlin"),
    ("php", "php"),
    ("python", "python"),
    ("ruby", "ruby"),
    ("rust", "rust"),
    ("scala", "scala"),
    ("shell", "bash"),
    ("swift", "swift"),
    ("typescript", "typescript"),
    ("zig", "zig"),
];

pub fn looks_like_tracks(root: &Path) -> bool {
    TRACKS
        .iter()
        .any(|(_, track)| root.join(track).join("exercises").join("practice").is_dir())
}

pub fn read(root: &Path) -> Result<Corpus, String> {
    if !looks_like_tracks(root) {
        return Err(format!(
            "{} does not hold Exercism track checkouts: expected <track>/exercises/practice",
            root.display()
        ));
    }

    let mut samples: BTreeMap<&'static str, Samples> = BTreeMap::new();
    let mut skipped = Vec::new();
    let mut units = BTreeMap::new();

    for (language, track) in TRACKS {
        let practice = root.join(track).join("exercises").join("practice");
        let Ok(entries) = fs::read_dir(&practice) else {
            skipped.push(format!("{language}: no track checkout at {track}/"));
            continue;
        };
        let Some(syntax) = comment_syntax(language) else {
            skipped.push(format!(
                "{language}: no comment syntax in the language profile"
            ));
            continue;
        };

        let mut paths = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        paths.sort();

        for exercise in paths {
            let Some(slug) = exercise.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            *units.entry(slug.to_owned()).or_insert(0usize) += 1;

            let Some(example) = example_files(&exercise) else {
                continue;
            };
            // An exercise may need more than one file. Summing them measures
            // the solution, not an arbitrary piece of it.
            let mut total = Measurement { lines: 0, bytes: 0 };
            for file in example {
                let path = exercise.join(&file);
                if !is_program_text(&path) {
                    continue;
                }
                let Ok(bytes) = fs::read(&path) else {
                    continue;
                };
                let measured = measure(&String::from_utf8_lossy(&bytes), syntax);
                total.lines += measured.lines;
                total.bytes += measured.bytes;
            }
            if !total.is_empty() {
                samples
                    .entry(language)
                    .or_default()
                    .insert(slug.to_owned(), total);
            }
        }
    }

    Ok(Corpus {
        revision: revisions(root),
        units: units.len(),
        samples,
        skipped,
    })
}

/// Whether a listed example file is program text rather than a manifest.
///
/// Deliberately keyed on the extension's *role*, not on the track's own
/// language profile. A C++ exercise's `example.h` belongs to Entl's `c` profile
/// — `.h` is C's extension, not C++'s — and asking "is this the track's
/// language?" would drop C++ headers while keeping C's, which would flatter C++
/// by exactly the size of its declarations. Asking "is this code?" counts both.
fn is_program_text(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(language_profile_for_extension)
        .is_some_and(|profile| profile.role == LanguageRole::Programming)
}

/// Reads `files.example` (or `files.exemplar`, which some tracks use) from an
/// exercise's `.meta/config.json`.
fn example_files(exercise: &Path) -> Option<Vec<String>> {
    let raw = fs::read_to_string(exercise.join(".meta").join("config.json")).ok()?;
    let config: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let files = config.get("files")?;
    let listed = files.get("example").or_else(|| files.get("exemplar"))?;
    let paths = listed
        .as_array()?
        .iter()
        .filter_map(|entry| entry.as_str())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    (!paths.is_empty()).then_some(paths)
}

/// One revision per track, since the corpus is a set of independent
/// repositories rather than a single checkout.
fn revisions(root: &Path) -> String {
    let mut parts = TRACKS
        .iter()
        .filter(|(_, track)| root.join(track).is_dir())
        .map(|(_, track)| {
            let revision = super::revision(&root.join(track));
            format!("{track}@{}", revision.get(..12).unwrap_or(&revision))
        })
        .collect::<Vec<_>>();
    parts.sort();
    parts.join(" ")
}
