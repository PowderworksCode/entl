//! Gate G0 for the C parser pack: parse every `.c`/`.h` under a root and
//! report what the pack could read.
//!
//! ```sh
//! cargo run --release -p entl-tree-sitter --example c_gate -- <packs-dir> <root>...
//! ```
//!
//! Per root: files parsed, hard failures, files clean as written, files clean
//! only after dialect rewrites (split by `rewrites_narrowed`), files that
//! still carry errors, and the worst offenders by error-node count.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{ParserCatalog, ParserRuntime};

fn c_files(root: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "c" || extension == "h")
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn count_errors(node: tree_sitter::Node) -> usize {
    let mut errors = usize::from(node.is_error() || node.is_missing());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        errors += count_errors(child);
    }
    errors
}

fn show_errors(parsed: &entl_tree_sitter::ParsedFile, limit: usize) {
    let text = String::from_utf8_lossy(&parsed.source);
    let lines: Vec<&str> = text.lines().collect();
    let mut sites = Vec::new();
    collect_error_lines(parsed.tree.root_node(), &mut sites);
    sites.dedup();
    for row in sites.iter().take(limit) {
        eprintln!(
            "  {}:{}: {}",
            parsed.path.display(),
            row + 1,
            lines.get(*row).unwrap_or(&"").trim()
        );
    }
}

fn collect_error_lines(node: tree_sitter::Node, out: &mut Vec<usize>) {
    if node.is_error() || node.is_missing() {
        out.push(node.start_position().row);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_lines(child, out);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let packs = PathBuf::from(arguments.next().expect("first argument: parser-packs dir"));
    let roots: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
    assert!(!roots.is_empty(), "at least one root to scan");

    let discovery = ParserCatalog::discover([packs]);
    let catalog = discovery.catalog;
    for problem in &discovery.errors {
        eprintln!("pack problem: {problem}");
    }
    let pack = catalog
        .resolve("c", std::path::Path::new("probe.c"))
        .expect("a C pack in the catalog");
    let runtime = ParserRuntime::new()?;
    let parser = runtime.load(Arc::clone(pack))?;

    for root in roots {
        let files = c_files(&root);
        let mut clean = 0usize;
        let mut rewritten_clean = 0usize;
        let mut rewritten_narrowed = 0usize;
        let mut failed = 0usize;
        let mut hard = 0usize;
        let mut worst: Vec<(usize, PathBuf)> = Vec::new();
        let mut reasons: std::collections::BTreeMap<&'static str, usize> =
            std::collections::BTreeMap::new();

        for path in &files {
            let Ok(source) = std::fs::read(path) else {
                hard += 1;
                continue;
            };
            let Ok(parsed) = parser.parse(path.clone(), source) else {
                hard += 1;
                continue;
            };
            if parsed.tree.root_node().has_error() {
                failed += 1;
                worst.push((count_errors(parsed.tree.root_node()), path.clone()));
                if std::env::var_os("C_GATE_SHOW").is_some() {
                    show_errors(&parsed, 3);
                }
            } else if parsed.rewrites.is_empty() {
                clean += 1;
            } else {
                rewritten_clean += 1;
                rewritten_narrowed += usize::from(parsed.rewrites_narrowed);
                for reason in &parsed.rewrites {
                    *reasons.entry(reason).or_insert(0) += 1;
                }
            }
        }

        let total = files.len();
        let readable = clean + rewritten_clean;
        println!("== {} ==", root.display());
        println!("files:                {total}");
        println!("hard failures:        {hard}");
        println!(
            "clean as written:     {clean}  ({:.1}%)",
            clean as f64 / total as f64 * 100.0
        );
        println!("clean after rewrite:  {rewritten_clean}  (narrowed in {rewritten_narrowed})");
        println!(
            "READABLE:             {readable}  ({:.1}%)",
            readable as f64 / total as f64 * 100.0
        );
        println!(
            "still failing:        {failed}  ({:.1}%)",
            failed as f64 / total as f64 * 100.0
        );
        if !reasons.is_empty() {
            println!("rewrite reasons:");
            for (reason, count) in &reasons {
                println!("  {count:>5}  {reason}");
            }
        }
        worst.sort_by_key(|(errors, _)| std::cmp::Reverse(*errors));
        if !worst.is_empty() {
            println!("worst files:");
            for (errors, path) in worst.iter().take(10) {
                println!("  {errors:>5}  {}", path.display());
            }
        }
        println!();
    }
    Ok(())
}
