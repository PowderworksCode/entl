use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use entl_codebase::{LanguageProfile, language_profile};
use sha2::{Digest, Sha256};

use crate::{Error, MANIFEST_FILENAME, ParserPackManifest, Result};

#[derive(Debug, Clone)]
pub struct ParserPack {
    directory: PathBuf,
    manifest: ParserPackManifest,
    language: &'static LanguageProfile,
    grammar: Arc<[u8]>,
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

        Ok(Self {
            directory: directory.to_path_buf(),
            manifest,
            language,
            grammar: grammar.into(),
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
            let mut children = entries
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.join(MANIFEST_FILENAME).is_file())
                .collect::<Vec<_>>();
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
