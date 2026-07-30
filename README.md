# Entl

Entl turns specific kinds of external data into typed Rust facts that can be
inspected, shaped, and moved without first forcing them through a database or a
stringly typed table interface.

The first domain is a source codebase's present tree. `entl-codebase` walks a
local checkout once and returns reusable facts about:

- files, language-detection evidence, and lexical comment syntax;
- Cargo and `package.json` packages;
- Cargo and JavaScript workspace membership;
- package managers inferred from declarations, workspace membership, and lockfiles;
- project boundaries, ecosystem roles, and language project signals;
- language-linked test-layout, inline-test, and required-config conventions;
- language-linked tool profiles and typed task classifications;
- direct dependency names and kinds;
- package scripts;
- distributable artifact instances for binaries, site bundles, napi-rs addons, and Tauri apps;
- the nearest package owners of every file; and
- recoverable inventory diagnostics.

The crate enforces no codebase policy. A linter can consume files and lazy
text, an auditor can consume package/workspace structure, and a later
tree-sitter adapter can turn identified source files into symbols and syntax
facts.

`entl-github` derives provider-specific facts from that reusable codebase
inventory. It recognizes GitHub Actions workflow files and triggers, expands
package scripts, and uses `entl-codebase` tool profiles to produce typed test,
lint, format, typecheck, and build invocations. It does not decide which tasks
policy requires. Build invocations retain typed artifact outputs and exact
package scopes, including Cargo workspace members.

There are two entry points. `walk` returns only the file layer—paths, sizes,
language evidence, diagnostics, and lazy content reads. `inspect` builds the
package and workspace facts on top of the same walk. Linked consumers can add
deterministic enrichment handlers through `discovery_registry`.

GitHub inspection is an explicit second step:

```rust
let codebase = entl_codebase::inspect(".", &entl_codebase::InventoryOptions::default())?;
let github = entl_github::inspect(&codebase);

for workflow in &github.workflows {
    println!("{}", workflow.path.display());
}
# Ok::<(), entl_codebase::Error>(())
```

```rust
use entl_codebase::{InventoryOptions, inspect};

let codebase = inspect(".", &InventoryOptions::default())?;

for package in &codebase.packages {
    println!("{} at {}", package.id, package.root.display());
    for file in codebase.files_for_package(&package.id) {
        if let Some(language) = &file.language {
            println!("  {} ({})", file.path.display(), language.language);
        }
    }
}
# Ok::<(), entl_codebase::Error>(())
```

A source scanner can avoid manifest parsing:

```rust
use entl_codebase::{InventoryOptions, walk};

let tree = walk(".", &InventoryOptions::default())?;
for file in tree.files_with_language("rust") {
    let source = tree.read_text(&file.path)?;
    println!("{}: {} bytes", file.path.display(), source.len());
}
# Ok::<(), entl_codebase::Error>(())
```

## Runtime Tree-sitter parsers

`entl-tree-sitter` loads versioned Wasm parser packs at runtime. Grammar
implementations are data artifacts rather than Rust dependencies. Verified
Rust, JavaScript, TypeScript, and TSX packs are included under `parser-packs`.
Each manifest declares which Entl language and file extensions it handles, so
multiple grammar variants can serve one language without consumer hardcoding.

The `grammar.wasm` artifacts are vendored third-party builds, not Entl's own
code. Each pack's `parser.toml` pins the upstream repository, revision, version,
license, and the artifact's `sha256`. Upstream licenses are reproduced in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Workspace

```text
crates/entl-codebase   typed codebase inventory and profiles
crates/entl-github     typed GitHub workflow and automation facts
crates/entl-tree-sitter runtime-loaded Wasm parser packs
parser-packs           pinned runtime grammar artifacts (third-party, vendored)
docs/design.md         boundaries and planned adapters
```

This is intentionally not yet a CLI, query engine, database, binding suite, or
generic ETL framework. See [the design](docs/design.md).

## Development

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## License

Entl's own source is [MIT](LICENSE). The vendored Tree-sitter grammars under
`parser-packs/` are third-party works under their own licenses; see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
