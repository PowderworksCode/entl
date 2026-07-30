use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::{
    CodebaseTree, Diagnostic, DiagnosticKind, Error, FileEntry, Result, TraversalDirectory,
    detect_language, traversal_directories,
};

#[derive(Debug, Clone)]
pub struct InventoryOptions {
    /// Honor codebase `.gitignore`, `.git/info/exclude`, and `.ignore` files.
    pub respect_gitignore: bool,
    /// Honor the user's global Git ignore file. Disabled by default so an
    /// inventory is reproducible across machines.
    pub respect_global_gitignore: bool,
    /// Discover ignore files above the requested root. Disabled by default so
    /// the root remains the complete context boundary.
    pub respect_parent_ignores: bool,
    /// Include conventional dependency and generated-output directories.
    pub include_generated: bool,
    /// Include hidden files and directories other than `.git`.
    pub include_hidden: bool,
    /// Extra codebase-relative glob patterns to omit.
    pub additional_ignores: Vec<String>,
    /// Read the first line of otherwise unidentified files to recognize
    /// interpreters such as `python`, `node`, and `bash`.
    pub sniff_shebangs: bool,
}

impl Default for InventoryOptions {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            respect_global_gitignore: false,
            respect_parent_ignores: false,
            include_generated: false,
            include_hidden: true,
            additional_ignores: Vec::new(),
            sniff_shebangs: true,
        }
    }
}

/// Walk one local codebase without parsing manifests or resolving package
/// relationships. This is the cheaper entry point for source scanners that
/// need paths, sizes, language evidence, and lazy content access only.
pub fn walk(root: impl AsRef<Path>, options: &InventoryOptions) -> Result<CodebaseTree> {
    let requested_root = root.as_ref();
    let root = requested_root
        .canonicalize()
        .map_err(|source| Error::root(requested_root, source))?;
    if !root.is_dir() {
        return Err(Error::root(
            &root,
            std::io::Error::new(std::io::ErrorKind::NotADirectory, "root is not a directory"),
        ));
    }

    let ignores = IgnoreMatcher::new(&options.additional_ignores)?;
    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(!options.include_hidden)
        .follow_links(false)
        .git_ignore(options.respect_gitignore)
        .git_exclude(options.respect_gitignore)
        .git_global(options.respect_global_gitignore)
        .ignore(options.respect_gitignore)
        .parents(options.respect_parent_ignores)
        .require_git(false);

    let filter_root = root.clone();
    let filter_ignores = ignores.clone();
    let include_generated = options.include_generated;
    let traversal_directories = traversal_directories();
    builder.filter_entry(move |entry| {
        if entry.depth() == 0 {
            return true;
        }
        let relative = entry
            .path()
            .strip_prefix(&filter_root)
            .unwrap_or(entry.path());
        if entry.file_type().is_some_and(|kind| kind.is_dir()) {
            let name = entry.file_name().to_string_lossy();
            if name == ".git"
                || (!include_generated
                    && traversal_directories.iter().any(|convention| {
                        convention.name == name
                            && directory_applies(convention, &filter_root, entry.path())
                    }))
            {
                return false;
            }
        }
        !filter_ignores.matches(relative)
    });

    let mut files = Vec::new();
    let mut diagnostics = Vec::new();
    for result in builder.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Walk,
                    path: PathBuf::new(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&root)
            .unwrap_or(entry.path())
            .to_path_buf();
        let size = match entry.metadata() {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Metadata,
                    path: relative.clone(),
                    message: error.to_string(),
                });
                0
            }
        };
        let mut language = detect_language(&relative, None);
        if language.is_none() && options.sniff_shebangs {
            match read_prefix(entry.path(), 512) {
                Ok(prefix) => language = detect_language(&relative, Some(&prefix)),
                Err(error) => diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Metadata,
                    path: relative.clone(),
                    message: format!("could not inspect file prefix: {error}"),
                }),
            }
        }
        files.push(FileEntry {
            path: relative,
            size,
            language,
            packages: Vec::new(),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.kind.cmp(&right.kind))
            .then(left.message.cmp(&right.message))
    });

    Ok(CodebaseTree {
        root,
        files,
        diagnostics,
    })
}

fn directory_applies(convention: &TraversalDirectory, root: &Path, directory: &Path) -> bool {
    if convention.markers.is_empty() {
        return true;
    }
    let mut ancestor = directory.parent();
    while let Some(path) = ancestor {
        if convention
            .markers
            .iter()
            .any(|marker| path.join(marker).is_file())
        {
            return true;
        }
        if path == root {
            break;
        }
        ancestor = path.parent();
    }
    false
}

#[derive(Clone)]
struct IgnoreMatcher {
    globs: GlobSet,
    prefixes: Vec<PathBuf>,
}

impl IgnoreMatcher {
    fn new(patterns: &[String]) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        let mut prefixes = Vec::new();
        for pattern in patterns {
            let normalized = pattern.trim_start_matches('/');
            let glob = Glob::new(normalized).map_err(|error| Error::IgnorePattern {
                pattern: pattern.clone(),
                message: error.to_string(),
            })?;
            builder.add(glob);
            if let Some(prefix) = normalized.strip_suffix("/**") {
                prefixes.push(PathBuf::from(prefix));
            }
        }
        let globs = builder.build().map_err(|error| Error::IgnorePattern {
            pattern: patterns.join(", "),
            message: error.to_string(),
        })?;
        Ok(Self { globs, prefixes })
    }

    fn matches(&self, path: &Path) -> bool {
        self.globs.is_match(path)
            || self
                .prefixes
                .iter()
                .any(|prefix| path == prefix || path.starts_with(prefix))
    }
}

fn read_prefix(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit).read_to_end(&mut bytes)?;
    Ok(bytes)
}
