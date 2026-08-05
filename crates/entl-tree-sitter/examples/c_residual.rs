//! For every C file a pack still cannot read, apply the dialect rewrites
//! unconditionally and report what fails in the *rewritten* text — the actual
//! blockers, not the constructs a discarded rewrite would have fixed.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{ParserCatalog, ParserRuntime, neutralize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let packs = PathBuf::from(arguments.next().expect("packs dir"));
    let root = PathBuf::from(arguments.next().expect("root"));

    let discovery = ParserCatalog::discover([packs]);
    let pack = discovery
        .catalog
        .resolve("c", std::path::Path::new("probe.c"))
        .expect("a C pack");
    let runtime = ParserRuntime::new()?;
    let parser = runtime.load(Arc::clone(pack))?;

    let mut stack = vec![root];
    let mut files = Vec::new();
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
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

    for path in files {
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        let parsed = parser.parse(path.clone(), source.clone())?;
        if !parsed.tree.root_node().has_error() {
            continue;
        }
        // The runtime discarded any rewrite that did not fully clean the
        // file; redo it by hand and look at what is left.
        let rewritten = neutralize("c", &source).map_or(source, |outcome| outcome.source);
        let reparsed = parser.parse(path.clone(), rewritten.clone())?;
        let text = String::from_utf8_lossy(&rewritten);
        let lines: Vec<&str> = text.lines().collect();
        let mut sites = Vec::new();
        collect(reparsed.tree.root_node(), &mut sites);
        sites.dedup();
        for row in sites.iter().take(2) {
            println!(
                "{}:{}: {}",
                path.display(),
                row + 1,
                lines.get(*row).unwrap_or(&"").trim()
            );
        }
    }
    Ok(())
}

fn collect(node: tree_sitter::Node, out: &mut Vec<usize>) {
    if node.is_error() || node.is_missing() {
        out.push(node.start_position().row);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, out);
    }
}
