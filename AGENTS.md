# AGENTS.md

## What Entl is

Entl acquires, shapes, and moves well-understood kinds of data through typed
Rust APIs. The first crate, `entl-codebase`, turns a source tree into facts
about files, languages, packages, projects, ecosystems, and workspaces.

Read [docs/design.md](docs/design.md) before changing the public model.

## Boundaries

- Keep domain types at the center. Arrow, SQL, JSONL, and other physical forms
  are adapters, not the canonical representation.
- Entl reports observed facts and evidence. Consumers own policy, findings,
  remediation, and presentation.
- Codebase-relative paths remain `PathBuf`; do not make UTF-8 a hidden
  filesystem requirement.
- Complete file content is lazy. Inventory may inspect a bounded prefix for
  classification but must not eagerly load the source tree.
- A package can exist outside a workspace, and a directory can contain package
  definitions from more than one ecosystem.
- Recoverable failures become scoped diagnostics. Only failure to establish a
  usable root or valid options is fatal.
- Do not add a CLI, database, language binding, or generic pipeline abstraction
  without a concrete consumer that needs it.

## Build and test

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Use `RUSTC_WRAPPER=` locally if a configured compiler cache is unavailable.

The example prints an inventory for manual inspection:

```sh
cargo run -p entl-codebase --example inspect -- PATH
```

## Working agreement

- Add fixture coverage for every new manifest, workspace, or language rule.
- Keep output deterministic: sorted facts, stable IDs, and no timestamps.
- Do not modify sibling consumer repositories while working in this repo.
- Do not commit, push, or publish unless asked.
