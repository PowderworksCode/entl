use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use entl::codebase::{LanguageProfile, language_profile};
use sha2::{Digest, Sha256};

use crate::{Error, MANIFEST_FILENAME, ParserPackManifest, Result};

/// Where a pack keeps its Tree-sitter queries, by convention rather than
/// manifest declaration, which is how the wider Tree-sitter ecosystem ships
/// them and is what makes an upstream grammar's queries usable as vendored.
const QUERY_DIRECTORY: &str = "queries";
const QUERY_EXTENSION: &str = "scm";

#[derive(Debug, Clone)]
pub struct ParserPack {
    directory: PathBuf,
    manifest: ParserPackManifest,
    language: &'static LanguageProfile,
    grammar: Arc<[u8]>,
    /// Query sources by name, which is the file stem. Compiling them needs a
    /// loaded grammar, so a pack carries the text and a parser carries the
    /// compiled form.
    queries: BTreeMap<String, Arc<str>>,
    queries_sha256: String,
}

impl ParserPack {
    pub fn load(directory: impl AsRef<Path>) -> Result<Self> {
        let directory = directory.as_ref();
        let manifest = ParserPackManifest::read(directory)?;
        let language =
            language_profile(&manifest.language).ok_or_else(|| Error::UnknownLanguage {
                pack: manifest.id.clone(),
                language: manifest.language.clone(),
            })?;
        let expected = normalize_digest(&manifest.id, &manifest.sha256)?;
        let grammar_path = directory.join(&manifest.grammar_path);
        let grammar = std::fs::read(&grammar_path).map_err(|source| Error::Read {
            path: grammar_path,
            source,
        })?;
        let actual = hex_digest(&grammar);
        if actual != expected {
            return Err(Error::DigestMismatch {
                pack: manifest.id.clone(),
                expected,
                actual,
            });
        }

        let queries = read_queries(directory)?;
        Ok(Self {
            directory: directory.to_path_buf(),
            manifest,
            language,
            grammar: grammar.into(),
            queries_sha256: queries_digest(&queries),
            queries,
        })
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn manifest(&self) -> &ParserPackManifest {
        &self.manifest
    }

    pub fn language(&self) -> &'static LanguageProfile {
        self.language
    }

    pub fn grammar(&self) -> &[u8] {
        &self.grammar
    }

    /// The query sources this pack ships, by name.
    pub fn queries(&self) -> &BTreeMap<String, Arc<str>> {
        &self.queries
    }

    /// A digest over every query this pack ships.
    ///
    /// A fact derived through a query depends on that query's text as much as
    /// on the grammar, so provenance that records only `sha256` cannot say
    /// which rules produced it.
    pub fn queries_sha256(&self) -> &str {
        &self.queries_sha256
    }

    pub fn matches(&self, path: &Path) -> bool {
        let files = &self.manifest.files;
        if files.extensions.is_empty() && files.filenames.is_empty() {
            return true;
        }
        let filename = path.file_name().and_then(|name| name.to_str());
        if filename.is_some_and(|filename| files.filenames.iter().any(|item| item == filename)) {
            return true;
        }
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .is_some_and(|extension| files.extensions.iter().any(|item| item == &extension))
    }

    fn is_fallback(&self) -> bool {
        self.manifest.files.extensions.is_empty() && self.manifest.files.filenames.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct ParserCatalog {
    packs: BTreeMap<String, Vec<Arc<ParserPack>>>,
}

impl ParserCatalog {
    pub fn discover(search_paths: impl IntoIterator<Item = PathBuf>) -> CatalogDiscovery {
        let mut discovery = CatalogDiscovery::default();
        let mut directories = Vec::new();

        for search_path in search_paths {
            if search_path.join(MANIFEST_FILENAME).is_file() {
                directories.push(search_path);
                continue;
            }
            let entries = match std::fs::read_dir(&search_path) {
                Ok(entries) => entries,
                Err(source) => {
                    discovery.errors.push(Error::Read {
                        path: search_path,
                        source,
                    });
                    continue;
                }
            };
            // An unreadable entry would otherwise drop a parser pack silently,
            // and a missing pack reads downstream as "this language has no
            // rules" rather than "this language was not looked at".
            let mut children = Vec::new();
            for entry in entries {
                match entry {
                    Ok(entry) => children.push(entry.path()),
                    Err(source) => discovery.errors.push(Error::Read {
                        path: search_path.clone(),
                        source,
                    }),
                }
            }
            children.retain(|path| path.join(MANIFEST_FILENAME).is_file());
            children.sort();
            directories.extend(children);
        }

        for directory in directories {
            match ParserPack::load(&directory) {
                Ok(pack) => {
                    let language = pack.language().id.to_owned();
                    let packs = discovery.catalog.packs.entry(language.clone()).or_default();
                    if let Some(first) = packs.iter().find(|first| selectors_overlap(first, &pack))
                    {
                        discovery.errors.push(Error::OverlappingPacks {
                            language,
                            first: first.manifest().id.clone(),
                            second: pack.manifest().id.clone(),
                        });
                        continue;
                    }
                    // A second grammar for one language must describe that
                    // language the same way. Where it does not, the same code
                    // gets a different answer depending on which file extension
                    // it was written under, and nothing downstream can see it:
                    // an analyzer that needs a query a pack does not ship skips
                    // the file in silence, because a pack describing no forms
                    // for its language is a real and different thing from a
                    // language having none.
                    //
                    // Unlike an overlap this does not make resolution
                    // ambiguous, so the pack is kept and the divergence is
                    // reported. Dropping it would lose the grammar outright for
                    // anyone who read past the errors.
                    for first in packs.iter() {
                        if let Some(difference) = describes_differently(first, &pack) {
                            discovery.errors.push(Error::DivergentPacks {
                                language: language.clone(),
                                first: first.manifest().id.clone(),
                                second: pack.manifest().id.clone(),
                                difference,
                            });
                        }
                    }
                    packs.push(Arc::new(pack));
                    packs.sort_by_key(|pack| pack.manifest().id.clone());
                }
                Err(error) => discovery.errors.push(error),
            }
        }

        discovery
    }

    pub fn resolve(&self, language: &str, path: &Path) -> Option<&Arc<ParserPack>> {
        let packs = self.packs.get(language)?;
        packs
            .iter()
            .find(|pack| !pack.is_fallback() && pack.matches(path))
            .or_else(|| packs.iter().find(|pack| pack.is_fallback()))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<ParserPack>> {
        self.packs.values().flatten()
    }

    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }
}

/// How two packs for one language disagree about what that language has, if
/// they do.
///
/// It compares what each pack CLAIMS TO DESCRIBE, not how it describes it. Two
/// grammars for one language legitimately need different patterns for the same
/// construct, so the query TEXT is free to differ; the set of query names is
/// not, because a missing name is a capability that silently does not run. The
/// error-handling manifest is not data about a grammar at all — it is a fact
/// about the language's standard library — so two packs for one language that
/// disagree about it disagree about the language itself.
fn describes_differently(first: &ParserPack, second: &ParserPack) -> Option<String> {
    let first_queries: BTreeSet<&str> = first.queries().keys().map(String::as_str).collect();
    let second_queries: BTreeSet<&str> = second.queries().keys().map(String::as_str).collect();
    if first_queries != second_queries {
        let only_first = first_queries.difference(&second_queries).copied();
        let only_second = second_queries.difference(&first_queries).copied();
        let mut missing = Vec::new();
        for (pack, names) in [
            (second.manifest().id.as_str(), only_first),
            (first.manifest().id.as_str(), only_second),
        ] {
            let names = names.collect::<Vec<_>>();
            if !names.is_empty() {
                missing.push(format!("{pack:?} ships no {}", names.join(", ")));
            }
        }
        return Some(missing.join("; "));
    }
    if first.manifest().error_handling != second.manifest().error_handling {
        return Some("they declare different [error-handling]".to_owned());
    }
    None
}

fn selectors_overlap(first: &ParserPack, second: &ParserPack) -> bool {
    let first = &first.manifest().files;
    let second = &second.manifest().files;
    let first_is_fallback = first.extensions.is_empty() && first.filenames.is_empty();
    let second_is_fallback = second.extensions.is_empty() && second.filenames.is_empty();
    (first_is_fallback && second_is_fallback)
        || first
            .extensions
            .iter()
            .any(|extension| second.extensions.contains(extension))
        || first
            .filenames
            .iter()
            .any(|filename| second.filenames.contains(filename))
        || first.filenames.iter().any(|filename| {
            filename_extension(filename)
                .is_some_and(|extension| second.extensions.contains(&extension))
        })
        || second.filenames.iter().any(|filename| {
            filename_extension(filename)
                .is_some_and(|extension| first.extensions.contains(&extension))
        })
}

fn filename_extension(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

#[derive(Debug, Default)]
pub struct CatalogDiscovery {
    pub catalog: ParserCatalog,
    pub errors: Vec<Error>,
}

/// Read every `queries/*.scm` a pack ships.
///
/// A pack with no query directory simply has no queries. A directory that
/// cannot be read, or a file in it that cannot be read, is an error: a query
/// that silently goes missing matches nothing, and a rule that matches nothing
/// reports nothing, which is indistinguishable from a clean repository.
fn read_queries(directory: &Path) -> Result<BTreeMap<String, Arc<str>>> {
    let root = directory.join(QUERY_DIRECTORY);
    if !root.is_dir() {
        return Ok(BTreeMap::new());
    }
    let entries = std::fs::read_dir(&root).map_err(|source| Error::Read {
        path: root.clone(),
        source,
    })?;
    let mut queries = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: root.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some(QUERY_EXTENSION) {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let source = std::fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        queries.insert(name.to_owned(), Arc::from(source.as_str()));
    }
    Ok(queries)
}

/// A digest over the query set, stable across filesystem ordering.
fn queries_digest(queries: &BTreeMap<String, Arc<str>>) -> String {
    let mut hasher = Sha256::new();
    for (name, source) in queries {
        hasher.update(name.as_bytes());
        hasher.update([0]);
        hasher.update(source.as_bytes());
        hasher.update([0]);
    }
    hex_digest_of(&hasher.finalize())
}

fn hex_digest_of(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn normalize_digest(pack: &str, digest: &str) -> Result<String> {
    let digest = digest.to_ascii_lowercase();
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidDigest {
            pack: pack.to_owned(),
            digest,
        });
    }
    Ok(digest)
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn loads_a_verified_pack() {
        let directory = tempdir().unwrap();
        let grammar = b"not a real grammar";
        fs::write(directory.path().join("grammar.wasm"), grammar).unwrap();
        fs::write(
            directory.path().join(MANIFEST_FILENAME),
            format!(
                "schema = 1\nid = \"rust-test\"\nlanguage = \"rust\"\nversion = \"1\"\nsource = \"https://example.com/rust\"\nrevision = \"abc\"\nlicense = \"MIT\"\nabi = 15\nsha256 = \"{}\"\ncomparison-domain = \"rust\"\n",
                hex_digest(grammar)
            ),
        )
        .unwrap();

        let pack = ParserPack::load(directory.path()).unwrap();
        assert_eq!(pack.language().id, "rust");
        assert_eq!(pack.grammar(), grammar);
    }

    #[test]
    fn rejects_unverified_bytes() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("grammar.wasm"), b"changed").unwrap();
        fs::write(
            directory.path().join(MANIFEST_FILENAME),
            "schema = 1\nid = \"rust-test\"\nlanguage = \"rust\"\nversion = \"1\"\nsource = \"https://example.com/rust\"\nrevision = \"abc\"\nlicense = \"MIT\"\nabi = 15\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\ncomparison-domain = \"rust\"\n",
        )
        .unwrap();

        assert!(matches!(
            ParserPack::load(directory.path()),
            Err(Error::DigestMismatch { .. })
        ));
    }
}
