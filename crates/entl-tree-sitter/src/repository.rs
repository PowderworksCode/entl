use std::path::{Path, PathBuf};
use std::sync::Arc;

use entl_codebase::{InventoryOptions, walk};

use crate::{ParsedFile, ParserCatalog, ParserRuntime, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub struct ParsedRepository {
    pub files: Vec<ParsedFile>,
    pub diagnostics: Vec<ParseDiagnostic>,
}

pub fn parse_repository(
    root: impl AsRef<Path>,
    catalog: &ParserCatalog,
) -> Result<ParsedRepository> {
    let tree = walk(root, &InventoryOptions::default())?;
    let runtime = ParserRuntime::new()?;
    let parsers = catalog
        .iter()
        .map(|pack| {
            runtime
                .load(pack.clone())
                .map(|parser| (pack.manifest().id.clone(), parser))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    let mut diagnostics = tree
        .diagnostics
        .iter()
        .map(|diagnostic| ParseDiagnostic {
            path: diagnostic.path.clone(),
            message: diagnostic.message.clone(),
        })
        .collect::<Vec<_>>();
    let mut files = Vec::new();

    for file in &tree.files {
        let Some(language) = file
            .language
            .as_ref()
            .map(|detection| detection.language.as_str())
        else {
            continue;
        };
        let Some(pack) = catalog.resolve(language, &file.path) else {
            continue;
        };
        let parser = parsers
            .get(&pack.manifest().id)
            .expect("a catalog pack always has a loaded parser");
        let source = match tree.read_bytes(&file.path) {
            Ok(source) => source,
            Err(error) => {
                diagnostics.push(ParseDiagnostic {
                    path: file.path.clone(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        let parsed = parser.parse(file.path.clone(), Arc::<[u8]>::from(source))?;
        if parsed.tree.root_node().has_error() {
            diagnostics.push(ParseDiagnostic {
                path: file.path.clone(),
                message: "Tree-sitter parse contains error nodes".to_owned(),
            });
            continue;
        }
        files.push(parsed);
    }

    Ok(ParsedRepository { files, diagnostics })
}
