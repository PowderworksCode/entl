//! How much of a real corpus a pack can actually read.
//!
//! A grammar rejects a whole file when any part of it is beyond what it knows,
//! so the number that matters before anything is built on a pack is not "does
//! it parse hello world" but "what fraction of a real tree does it lose". This
//! reports that, and names the files, so a rewrite rule can be written against
//! the corpus rather than against a guess.
//!
//! ```sh
//! cargo run -p entl-tree-sitter --example parse-rate -- parser-packs/python /usr/lib/python3.13
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use entl_tree_sitter::{ParsedFile, ParserPack, ParserRuntime};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let pack_path = PathBuf::from(arguments.next().ok_or("usage: parse-rate PACK ROOT...")?);
    let roots: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
    if roots.is_empty() {
        return Err("usage: parse-rate PACK ROOT...".into());
    }

    let pack = Arc::new(ParserPack::load(&pack_path)?);
    let extensions = pack.manifest().files.extensions.clone();
    let parser = ParserRuntime::new()?.load(pack)?;

    let mut totals = Totals::default();
    let mut failures = Vec::new();
    let mut rewritten = Vec::new();
    for root in &roots {
        let mut candidates = Vec::new();
        collect(root, &extensions, &mut candidates)?;
        candidates.sort();
        for path in candidates {
            let Ok(source) = std::fs::read(&path) else {
                totals.unreadable += 1;
                continue;
            };
            let parsed = parser.parse(path.clone(), Arc::<[u8]>::from(source))?;
            totals.files += 1;
            totals.bytes += parsed.source.len();
            totals.units += units(&parsed);
            totals.unreadable_bytes += unreadable_bytes(&parsed);
            if !parsed.rewrites.is_empty() {
                totals.rewritten += 1;
                rewritten.push(path.clone());
            }
            if parsed.tree.root_node().has_error() {
                totals.errored += 1;
                totals.dropped_bytes += parsed.source.len();
                failures.push((path.clone(), first_error(&parsed)));
            }
        }
    }

    println!("pack      {}", pack_path.display());
    println!("files     {}", totals.files);
    println!("bytes     {}", totals.bytes);
    println!("units     {}", totals.units);
    println!(
        "errored   {} ({:.2}%)",
        totals.errored,
        percentage(totals.errored, totals.files)
    );
    // The two are far apart and only one of them is what a consumer loses.
    //
    // `unreadable` is what the GRAMMAR could not read: the bytes inside error
    // nodes. `dropped` is what a CONSUMER never sees, because
    // `parse_repository` admits a file only when its tree is completely clean
    // and skips it whole otherwise. On CPython `main` the first is 0.05% and
    // the second is 2.6% — a fifty-fold difference, and the larger one is the
    // one that decides whether facts about a file exist at all.
    println!(
        "unreadable {} bytes ({:.4}% of source, inside error nodes)",
        totals.unreadable_bytes,
        percentage(totals.unreadable_bytes, totals.bytes)
    );
    println!(
        "dropped   {} bytes ({:.2}% of source, whole files a consumer never sees)",
        totals.dropped_bytes,
        percentage(totals.dropped_bytes, totals.bytes)
    );
    println!("rewritten {}", totals.rewritten);
    if totals.unreadable > 0 {
        println!("unreadable {}", totals.unreadable);
    }

    if !rewritten.is_empty() {
        println!("\n-- rewritten --");
        for path in &rewritten {
            println!("{}", path.display());
        }
    }

    if !failures.is_empty() {
        println!("\n-- errored, with the first error node's text --");
        for (path, excerpt) in &failures {
            println!("{}\n    {excerpt}", path.display());
        }
        println!("\n-- errored, grouped by first line of the error --");
        let mut grouped: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, excerpt) in &failures {
            *grouped.entry(excerpt.as_str()).or_default() += 1;
        }
        let mut ranked: Vec<_> = grouped.into_iter().collect();
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(right.0)));
        for (excerpt, count) in ranked.iter().take(40) {
            println!("{count:5}  {excerpt}");
        }
    }
    Ok(())
}

#[derive(Default)]
struct Totals {
    files: usize,
    bytes: usize,
    units: usize,
    errored: usize,
    rewritten: usize,
    unreadable: usize,
    unreadable_bytes: usize,
    dropped_bytes: usize,
}

fn percentage(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    (part as f64) * 100.0 / (whole as f64)
}

/// Named top-level children, which is what a grammar hole costs when it
/// swallows one.
fn units(parsed: &ParsedFile) -> usize {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .filter(|node| node.kind() != "ERROR")
        .count()
}

/// Bytes covered by the outermost ERROR nodes.
///
/// This is what a grammar hole COSTS, which is not the same as how often it
/// happens. Zig's conditional-type hole took the whole file with it; a Python
/// hole is usually bounded by the statement, because the grammar is delimited
/// by newlines and indentation rather than by braces. Only this number tells
/// the two apart, and it decides whether a rewrite is worth writing.
fn unreadable_bytes(parsed: &ParsedFile) -> usize {
    let mut cursor = parsed.tree.walk();
    let mut stack = vec![parsed.tree.root_node()];
    let mut total = 0;
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            total += node.end_byte() - node.start_byte();
            continue;
        }
        for child in node.children(&mut cursor).collect::<Vec<_>>() {
            if child.has_error() {
                stack.push(child);
            }
        }
    }
    total
}

/// The first line of the first ERROR node, which is usually the construct the
/// grammar could not read.
fn first_error(parsed: &ParsedFile) -> String {
    let mut cursor = parsed.tree.walk();
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let text = std::str::from_utf8(&parsed.source[node.start_byte()..node.end_byte()])
                .unwrap_or("<non-utf8>");
            let line = text.lines().next().unwrap_or_default().trim();
            return format!("{}:{} {}", node.start_position().row + 1, node.kind(), line);
        }
        for child in node
            .children(&mut cursor)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            if child.has_error() {
                stack.push(child);
            }
        }
    }
    "<no error node>".to_owned()
}

fn collect(root: &Path, extensions: &[String], into: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if root.is_file() {
        into.push(root.to_path_buf());
        return Ok(());
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            collect(&path, extensions, into)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.iter().any(|wanted| wanted == extension))
        {
            into.push(path);
        }
    }
    Ok(())
}
