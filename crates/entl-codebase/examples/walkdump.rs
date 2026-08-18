//! Dump path -> language for a walk of a directory, for before/after diffing.
fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let tree =
        entl_codebase::walk(&root, &entl_codebase::InventoryOptions::default()).expect("walk");
    for file in &tree.files {
        let lang = file
            .language
            .as_ref()
            .map(|d| d.language.as_str().to_string())
            .unwrap_or_else(|| "-".into());
        println!("{}\t{}", file.path.display(), lang);
    }
}
