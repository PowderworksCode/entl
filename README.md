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
- direct dependency names, kinds, declared sources, and version requirements;
- exact package versions, registry sources, and checksums observed in Cargo lockfiles;
- package scripts;
- distributable artifact instances for binaries, site bundles, napi-rs addons, and Tauri apps;
- the nearest package owners of every file; and
- recoverable inventory diagnostics.

Explicit compiler observation is separate from passive inventory. Consumers
that need build-context facts can call `observe_rust_compiler`; it runs the
active `rustc` from the repository directory and returns its exact release,
commit, host, sysroot, installed standard-library source location, cfg values,
and target features. A normal `inspect` never runs a compiler.

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
Parser packs also declare language-specific syntax-unit node kinds used to
group token comparisons at function, method, implementation, and class
boundaries.

The `grammar.wasm` artifacts are vendored third-party builds, not Entl's own
code. Each pack's `parser.toml` pins the upstream repository, revision, version,
license, and the artifact's `sha256`. Upstream licenses are reproduced in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Workspace

```text
crates/entl-codebase   typed codebase inventory and profiles
crates/entl-github     typed GitHub workflow and automation facts
crates/entl-semantics  span-anchored semantic observations, language neutral
crates/entl-rust-mir   observes resolved Rust semantics by running as the compiler
crates/entl-tree-sitter runtime-loaded Wasm parser packs
parser-packs           pinned runtime grammar artifacts (third-party, vendored)
tools/rosetta-verbosity regenerates the language verbosity table from a corpus
docs/design.md         boundaries and planned adapters
```

This is intentionally not yet a CLI, query engine, database, binding suite, or
generic ETL framework. See [the design](docs/design.md).

## Semantic observations

Syntax cannot say where a call goes. `use std::fs; fs::read(path)` and
`std::fs::read(path)` are one call written two ways, and only name resolution
knows it. `entl-semantics` defines what a compiler or language server can be
asked about a place in the source: what a name refers to, what type an
expression has, where a call goes, what a type implements. Every observation is
optional, and `Coverage` records which questions a provider attempted, so a
consumer can tell "nothing found" from "not looked at".

The schema deliberately holds no intermediate representation. Compilers
disagree at that level — some expose a control flow graph, some a typed syntax
tree, some neither — and unifying those yields something less useful than any of
them. Unifying the answers they can all give does not.

`entl-rust-mir` is the first provider. It replaces `rustc` for one compilation
and reads the resolved mid-level representation:

```sh
cd crates/entl-rust-mir && cargo build
ENTL_RUST_MIR_OUTPUT=/tmp/observations \
  target/debug/entl-rust-mir --crate-type lib --crate-name mycrate src/lib.rs
```

It lives outside the workspace on a pinned nightly with the compiler's private
crates. That isolation is the point: compiler-backed observations sharpen
results where a toolchain is available, and Tree-sitter remains the floor
everywhere else. A language whose compiler is not integrated still parses.

## Language verbosity

Language profiles carry a measured fact about how much source text a language
needs: `LanguageProfile::verbosity` for one language against the baseline, and
`verbosity_ratio` for a pair as it was actually measured.

```rust
use entl_codebase::{language_profile, verbosity_ratio};

let java = language_profile("java").unwrap().verbosity().unwrap();
let python = language_profile("python").unwrap().verbosity().unwrap();
assert!(java.bytes > python.bytes);

// Measured on the tasks both implement, not derived from the two indexes.
let measured = verbosity_ratio("java", "python").unwrap();
assert!(measured.tasks > 1000);
```

The numbers come from comparing Entl's languages on the
[Rosetta Code](https://rosettacode.org) corpus, on the tasks each pair has in
common. Because no two pairs share a task set, the ratios are not transitive,
and the single index per language is a fit rather than a fact — each profile
reports how far off that fit gets. See [docs/verbosity.md](docs/verbosity.md)
for the method and its limits, and `tools/rosetta-verbosity` to regenerate it.
Rosetta Code's own content is GFDL 1.2 and is not redistributed here; only the
measurements are.

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
