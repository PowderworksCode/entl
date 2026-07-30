use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use entl_codebase::CodebaseInventory;

use crate::{CodeownersConfiguration, CodeownersInventory, CodeownersRule, Diagnostic};

const MAX_SIZE: u64 = 3 * 1024 * 1024;
const LOCATIONS: [&str; 3] = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];

pub(crate) fn inspect(codebase: &CodebaseInventory) -> CodeownersInventory {
    let files = LOCATIONS
        .into_iter()
        .map(PathBuf::from)
        .filter(|path| codebase.has_file(path))
        .collect::<BTreeSet<_>>();
    let Some(path) = LOCATIONS
        .into_iter()
        .map(PathBuf::from)
        .find(|path| files.contains(path))
    else {
        return CodeownersInventory {
            files,
            ..CodeownersInventory::default()
        };
    };

    let mut diagnostics = Vec::new();
    let configuration = if codebase
        .file(&path)
        .is_some_and(|file| file.size > MAX_SIZE)
    {
        diagnostics.push(Diagnostic {
            path: path.clone(),
            message: "CODEOWNERS exceeds GitHub's 3 MB size limit".to_owned(),
        });
        None
    } else {
        match codebase.read_text(&path) {
            Ok(text) => Some(parse(&path, &text, &mut diagnostics)),
            Err(error) => {
                diagnostics.push(Diagnostic {
                    path: path.clone(),
                    message: format!("CODEOWNERS is unreadable: {error}"),
                });
                None
            }
        }
    };

    CodeownersInventory {
        files,
        configuration,
        diagnostics,
    }
}

fn parse(path: &Path, text: &str, diagnostics: &mut Vec<Diagnostic>) -> CodeownersConfiguration {
    let mut rules = Vec::new();
    for (index, source) in text.lines().enumerate() {
        let line = index + 1;
        let mut fields = source
            .split_whitespace()
            .take_while(|field| !field.starts_with('#'));
        let Some(pattern) = fields.next() else {
            continue;
        };
        if let Some(message) = invalid_pattern(pattern) {
            diagnostics.push(line_diagnostic(path, line, message));
            continue;
        }
        let owners = fields.map(str::to_owned).collect::<Vec<_>>();
        if let Some(owner) = owners.iter().find(|owner| !valid_owner(owner)) {
            diagnostics.push(line_diagnostic(
                path,
                line,
                format!("invalid owner `{owner}`"),
            ));
            continue;
        }
        rules.push(CodeownersRule {
            line,
            pattern: pattern.to_owned(),
            owners,
        });
    }
    CodeownersConfiguration {
        path: path.to_path_buf(),
        rules,
    }
}

fn invalid_pattern(pattern: &str) -> Option<&'static str> {
    if pattern.starts_with('!') {
        Some("negated patterns are not supported by GitHub CODEOWNERS")
    } else if pattern.contains('[') || pattern.contains(']') {
        Some("character ranges are not supported by GitHub CODEOWNERS")
    } else if pattern.starts_with("\\#") {
        Some("a leading # cannot be escaped in GitHub CODEOWNERS")
    } else {
        None
    }
}

fn valid_owner(owner: &str) -> bool {
    if let Some(account) = owner.strip_prefix('@') {
        let mut segments = account.split('/');
        let first = segments.next().is_some_and(valid_account_segment);
        let second = segments.next().is_none_or(valid_account_segment);
        return first && second && segments.next().is_none() && !account.contains('@');
    }
    let Some((local, domain)) = owner.split_once('@') else {
        return false;
    };
    !local.is_empty() && !domain.is_empty() && !domain.contains('@')
}

fn valid_account_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn line_diagnostic(path: &Path, line: usize, message: impl AsRef<str>) -> Diagnostic {
    Diagnostic {
        path: path.to_path_buf(),
        message: format!("CODEOWNERS line {line}: {}", message.as_ref()),
    }
}
