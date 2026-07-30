use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{ParserCatalog, ParserRuntime};

#[test]
fn discovers_and_selects_web_parser_packs() {
    let packs = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("parser-packs");
    let discovery = ParserCatalog::discover([packs]);
    assert!(discovery.errors.is_empty(), "{:?}", discovery.errors);
    let catalog = discovery.catalog;

    assert_eq!(
        catalog
            .resolve("javascript", PathBuf::from("src/app.jsx").as_path())
            .unwrap()
            .manifest()
            .id,
        "tree-sitter-javascript"
    );
    assert_eq!(
        catalog
            .resolve("typescript", PathBuf::from("src/app.ts").as_path())
            .unwrap()
            .manifest()
            .id,
        "tree-sitter-typescript"
    );
    assert_eq!(
        catalog
            .resolve("typescript", PathBuf::from("src/app.tsx").as_path())
            .unwrap()
            .manifest()
            .id,
        "tree-sitter-tsx"
    );
}

#[test]
fn parses_javascript_typescript_and_tsx() {
    let packs = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("parser-packs");
    let discovery = ParserCatalog::discover([packs]);
    assert!(discovery.errors.is_empty(), "{:?}", discovery.errors);
    let runtime = ParserRuntime::new().unwrap();

    for (language, path, source, expected_pack) in [
        (
            "javascript",
            "src/app.jsx",
            "export const App = () => <main>Hello</main>;",
            "tree-sitter-javascript",
        ),
        (
            "typescript",
            "src/value.ts",
            "export const value: number = 42;",
            "tree-sitter-typescript",
        ),
        (
            "typescript",
            "src/app.tsx",
            "export const App = (): JSX.Element => <main>Hello</main>;",
            "tree-sitter-tsx",
        ),
    ] {
        let pack = discovery.catalog.resolve(language, path.as_ref()).unwrap();
        let parsed = runtime
            .load(pack.clone())
            .unwrap()
            .parse(path, Arc::<[u8]>::from(source.as_bytes()))
            .unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{path}");
        assert_eq!(parsed.provenance.parser_id, expected_pack);
    }
}
