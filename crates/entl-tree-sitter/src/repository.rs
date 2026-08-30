use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use entl::codebase::{InventoryOptions, walk};

use crate::{LoadedParser, ParsedFile, ParserCatalog, ParserRuntime, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug)]
pub struct ParsedRepository {
    pub files: Vec<ParsedFile>,
    pub diagnostics: Vec<ParseDiagnostic>,
    /// The loaded parsers, by pack id, so a consumer can run a pack's queries.
    ///
    /// A compiled query belongs to the grammar it was compiled against, so it
    /// cannot live on the pack, which is shared across runtimes.
    pub parsers: BTreeMap<String, LoadedParser>,
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
        .collect::<Result<BTreeMap<_, _>>>()?;
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
        let Some(detection) = file.language.as_ref() else {
            continue;
        };
        let language = detection.language.as_str();
        // A file whose language is known but has no pack is the one skip that
        // used to be silent, and it is the one that most needs saying: every
        // other path out of this loop records a diagnostic, so a consumer could
        // report a repository as clean when an entire language went unread.
        // It was Python's own state until a pack existed, and it is the state
        // of every language this fleet has not onboarded.
        //
        // Only for a role that expects a pack, though. Saying it for every
        // detected language made a README, a YAML file, and the very
        // `straitjacket.toml` a run was configured by each report as unread —
        // three findings that name nothing a consumer would parse, burying the
        // Go and C++ gaps this is meant to surface.
        let Some(pack) = catalog.resolve(language, &file.path) else {
            if detection
                .profile()
                .is_some_and(|profile| profile.role.expects_parser_pack())
            {
                diagnostics.push(ParseDiagnostic {
                    path: file.path.clone(),
                    message: format!("no {language} parser pack is configured, so nothing read it"),
                });
            }
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
                message: first_error(&parsed),
            });
            continue;
        }
        files.push(parsed);
    }

    Ok(ParsedRepository {
        files,
        diagnostics,
        parsers,
    })
}

/// Where a parse first went wrong, and what the source says there.
///
/// A grammar that rejects a file rejects all of it, so knowing which construct
/// defeated it is the difference between "this file was skipped" and knowing
/// what to do about it.
fn first_error(parsed: &ParsedFile) -> String {
    let mut cursor = parsed.tree.walk();
    let mut stack = vec![parsed.tree.root_node()];
    let mut earliest: Option<tree_sitter::Node<'_>> = None;
    while let Some(node) = stack.pop() {
        // An error node swallows everything it could not make sense of, so the
        // outermost one spans the file and says nothing. The narrowest one is
        // the closest thing to the construct that actually defeated the parse.
        let width = |node: tree_sitter::Node<'_>| node.end_byte() - node.start_byte();
        if (node.is_error() || node.is_missing())
            && earliest.is_none_or(|found| width(node) < width(found))
        {
            earliest = Some(node);
        }
        stack.extend(node.children(&mut cursor));
    }
    let Some(node) = earliest else {
        return "Tree-sitter parse contains error nodes".to_owned();
    };
    let line = node.start_position().row + 1;
    let excerpt = parsed
        .source
        .get(node.byte_range())
        .and_then(|bytes| std::str::from_utf8(bytes).ok()) // straitjacket-allow:error-discard — an excerpt for an error message degrades to empty
        .unwrap_or_default()
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(60)
        .collect::<String>();
    format!("line {line}: the grammar cannot read `{excerpt}`")
}
