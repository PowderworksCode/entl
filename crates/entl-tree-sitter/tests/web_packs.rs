#![allow(clippy::unwrap_used, clippy::expect_used)]
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

fn parser_packs() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("parser-packs")
}

/// `.ts` and `.tsx` are one language read by two grammars, so the two packs
/// must claim to describe the same things.
///
/// This is not a tidiness rule. `infact-errors` runs on a file only when its
/// pack ships `callables` and `discards`, and skips it in silence otherwise —
/// correctly, because a pack describing no discard forms is different from a
/// language having none. That reasoning holds for a LANGUAGE and fails for a
/// GRAMMAR: before this, identical bytes in `a.ts` and `a.tsx` produced three
/// findings and zero, with nothing anywhere saying the second file had been
/// read by a pack that could not look. The `[error-handling]` half is the
/// quieter one — with the queries copied across but not the manifest, the same
/// code came back "is fallible" under one extension and "is infallible" under
/// the other.
#[test]
fn packs_for_one_language_describe_it_the_same_way() {
    let discovery = ParserCatalog::discover([parser_packs()]);
    assert!(discovery.errors.is_empty(), "{:?}", discovery.errors);

    // Asserted directly as well as through the absence of errors: two packs
    // that both shipped nothing would agree perfectly and describe nothing.
    for id in ["tree-sitter-typescript", "tree-sitter-tsx"] {
        let pack = discovery
            .catalog
            .iter()
            .find(|pack| pack.manifest().id == id)
            .unwrap_or_else(|| panic!("no {id} pack"));
        for query in ["callables", "discards"] {
            assert!(
                pack.queries().contains_key(query),
                "{id} ships no {query} query, so TypeScript written in its \
                 extensions gets no discard analysis and nothing says so"
            );
        }
        assert_eq!(
            pack.manifest().error_handling.propagation,
            entl_tree_sitter::Propagation::Unchecked,
            "{id} must agree that any TypeScript callable can throw"
        );
    }
}

/// The guard above passes if the check does nothing, so the check is exercised.
#[test]
fn a_second_grammar_that_describes_less_is_reported() {
    let directory = tempfile::tempdir().unwrap();
    for pack in ["typescript", "tsx"] {
        let root = directory.path().join(pack);
        std::fs::create_dir_all(root.join("queries")).unwrap();
        for entry in std::fs::read_dir(parser_packs().join(pack)).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
            }
        }
        for entry in std::fs::read_dir(parser_packs().join(pack).join("queries")).unwrap() {
            let entry = entry.unwrap();
            // The divergence under test: the second grammar keeps only the
            // scaffolding, exactly as it stood before this parity work.
            if pack == "tsx" && entry.file_name() == "discards.scm" {
                continue;
            }
            std::fs::copy(entry.path(), root.join("queries").join(entry.file_name())).unwrap();
        }
    }

    let discovery = ParserCatalog::discover([directory.path().to_path_buf()]);
    let reported = discovery
        .errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert!(
        reported
            .iter()
            .any(|error| error.contains("tree-sitter-tsx") && error.contains("discards")),
        "a grammar shipping fewer queries than its sibling must be named: {reported:?}"
    );
    // Reporting it must not cost the grammar: resolution is unaffected by the
    // divergence, and dropping the pack would lose `.tsx` outright for anyone
    // who read past the errors.
    assert!(
        discovery
            .catalog
            .resolve("typescript", PathBuf::from("src/app.tsx").as_path())
            .is_some(),
        "the divergent pack must still be usable"
    );
}

/// JavaScript describes the same discard forms as TypeScript, and they fire.
///
/// A query that compiles is not a query attached to anything: a `#eq?` placed
/// after a pattern's closing paren compiles cleanly and silently becomes its own
/// pattern matching every node. So this runs the queries rather than loading
/// them, and asserts the negative cases too — every form here turns on the
/// ABSENCE of a binding, which is the one thing a query cannot state directly.
#[test]
fn the_javascript_pack_recognizes_its_discard_forms() {
    let discovery = ParserCatalog::discover([parser_packs()]);
    assert!(discovery.errors.is_empty(), "{:?}", discovery.errors);
    let pack = discovery
        .catalog
        .resolve("javascript", PathBuf::from("src/read.js").as_path())
        .unwrap();
    let parser = ParserRuntime::new().unwrap().load(pack.clone()).unwrap();

    let source = "\
export async function discards(path) {
  try { return await load(path); } catch { }
  void load(path);
  load(path).catch(() => null);
}
export async function handles(path) {
  try { return await load(path); } catch (error) { report(error); }
  return load(path).catch((error) => report(error));
}
";
    let file = parser
        .parse("src/read.js", Arc::<[u8]>::from(source.as_bytes()))
        .unwrap();
    assert!(!file.tree.root_node().has_error());

    // `{capture}.bind` is the name `infact-errors` tests for, so the test asks
    // the question the consumer asks rather than a convenient approximation.
    let found = parser.matches("discards", &file).unwrap();
    for form in [
        "discard.err-arm",
        "discard.let-underscore",
        "discard.ok-discard",
    ] {
        let bound = format!("{form}.bind");
        let discarded = found
            .iter()
            .filter(|found| found.has(form) && !found.has(&bound))
            .count();
        assert_eq!(
            discarded, 1,
            "{form} discarded {discarded} times: {found:?}"
        );
    }
    // `catch (error)` and `.catch((error) => ..)` still see the cause. They
    // match the same patterns and are told apart only by the binding capture
    // being present, so counting matches alone would pass while reporting
    // handled code as discarded.
    let bound = found
        .iter()
        .filter(|found| found.has("discard.err-arm.bind") || found.has("discard.ok-discard.bind"))
        .count();
    assert_eq!(bound, 2, "the two handled forms must bind: {found:?}");

    // The scaffolding has to reach a method on a class, or a discard inside one
    // is attributed to nothing. `class_declaration` names an `identifier` here
    // where TypeScript names a `type_identifier`, and getting that wrong fails
    // the pack load rather than losing the container quietly.
    let file = parser
        .parse(
            "src/read.js",
            Arc::<[u8]>::from(
                "export class Reader { async read(p) { return load(p); } }".as_bytes(),
            ),
        )
        .unwrap();
    let scaffold = parser.matches("callables", &file).unwrap();
    assert!(
        scaffold.iter().any(|found| found.has("impl.type")),
        "the class must be captured as the container: {scaffold:?}"
    );
    assert!(
        scaffold.iter().any(|found| found.has("callable.name")),
        "the method must be captured as a callable: {scaffold:?}"
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
