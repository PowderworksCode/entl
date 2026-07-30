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
