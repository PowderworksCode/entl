use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::{Error, Result};

pub const MANIFEST_FILENAME: &str = "parser.toml";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserPackManifest {
    pub schema: u32,
    pub id: String,
    pub language: String,
    #[serde(default, rename = "grammar-name")]
    pub grammar_name: Option<String>,
    pub version: String,
    pub source: String,
    pub revision: String,
    pub license: String,
    pub abi: usize,
    pub sha256: String,
    #[serde(rename = "comparison-domain")]
    pub comparison_domain: String,
    #[serde(default = "default_grammar_path", rename = "grammar-path")]
    pub grammar_path: PathBuf,
    #[serde(default)]
    pub files: FileSelectionManifest,
    #[serde(default)]
    pub tokenization: TokenizationManifest,
    #[serde(default, rename = "error-handling")]
    pub error_handling: ErrorHandlingManifest,
    #[serde(default)]
    pub tests: TestManifest,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSelectionManifest {
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenizationManifest {
    #[serde(default, rename = "ignored-node-kinds")]
    pub ignored_node_kinds: Vec<String>,
    #[serde(default, rename = "identifier-node-kinds")]
    pub identifier_node_kinds: Vec<String>,
    #[serde(default, rename = "literal-node-kinds")]
    pub literal_node_kinds: Vec<String>,
    /// Named syntax nodes that form useful whole-code comparison units.
    #[serde(default, rename = "unit-node-kinds")]
    pub unit_node_kinds: Vec<String>,
}

/// How a language spells failure.
///
/// These are facts about a language's standard library, not shapes in its
/// grammar, which is why they are data here rather than Tree-sitter queries. A
/// query can find `.unwrap_or_default()`; only this can say that the same
/// spelling reads identically on a type that never carried a failure, so a
/// consumer reporting it must say "possible" rather than "certain".
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorHandlingManifest {
    /// Type names that, named in a return type, mean a failure can be reported.
    #[serde(default, rename = "fallible-types")]
    pub fallible_types: Vec<String>,
    /// Type names that carry an absence rather than a cause.
    #[serde(default, rename = "optional-types")]
    pub optional_types: Vec<String>,
    /// Callables whose failure case is an ANSWER rather than a failure.
    ///
    /// `binary_search` reports "not present" and `strip_prefix` reports "not
    /// under this prefix". Both are ordinary results a caller is expected to
    /// discard, and reporting them buries the findings that matter.
    #[serde(default, rename = "non-failure-results")]
    pub non_failure_results: Vec<String>,
    /// The forms `non-failure-results` filters.
    ///
    /// A form only takes the exclusion when it identifies the discard BY the
    /// failure type: `.ok()` and `Ok(..)` name the `Result` itself, whereas
    /// `.unwrap_or(..)` says nothing about what it unwrapped.
    #[serde(default, rename = "non-failure-results-forms")]
    pub non_failure_results_forms: Vec<String>,
    /// Discard forms whose receiver type the syntax cannot decide.
    ///
    /// Named by the consumer's own form vocabulary, because which spellings are
    /// ambiguous is a fact about this language's standard library.
    #[serde(default, rename = "ambiguous-forms")]
    pub ambiguous_forms: Vec<String>,
}

/// How a language marks code that only runs under test.
///
/// The substrings are matched against whatever annotation text a pack's queries
/// capture — attributes in Rust, decorators or naming conventions elsewhere.
/// The query decides where to look; this decides what counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestManifest {
    /// Substrings marking a single item as a test.
    #[serde(default)]
    pub markers: Vec<String>,
    /// Substrings marking a whole module or file as test-only.
    #[serde(default, rename = "module-markers")]
    pub module_markers: Vec<String>,
}

impl ParserPackManifest {
    pub(crate) fn read(directory: &Path) -> Result<Self> {
        let path = directory.join(MANIFEST_FILENAME);
        let source = std::fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        let manifest = toml::from_str::<Self>(&source).map_err(|source| Error::Manifest {
            path: path.clone(),
            source,
        })?;
        if manifest.schema != 1 {
            return Err(Error::UnsupportedSchema {
                pack: manifest.id.clone(),
                schema: manifest.schema,
            });
        }
        validate_relative(&manifest.grammar_path)?;
        validate_file_selection(&manifest)?;
        Ok(manifest)
    }

    pub fn grammar_name(&self) -> &str {
        self.grammar_name.as_deref().unwrap_or(&self.language)
    }
}

fn default_grammar_path() -> PathBuf {
    PathBuf::from("grammar.wasm")
}

fn validate_relative(path: &Path) -> Result<()> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(Error::UnsafePath {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn validate_file_selection(manifest: &ParserPackManifest) -> Result<()> {
    for extension in &manifest.files.extensions {
        if extension.is_empty()
            || extension.starts_with('.')
            || extension != &extension.to_ascii_lowercase()
        {
            return Err(Error::InvalidFileSelector {
                pack: manifest.id.clone(),
                selector: extension.clone(),
            });
        }
    }
    for filename in &manifest.files.filenames {
        if filename.is_empty()
            || Path::new(filename)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(filename)
        {
            return Err(Error::InvalidFileSelector {
                pack: manifest.id.clone(),
                selector: filename.clone(),
            });
        }
    }
    Ok(())
}
