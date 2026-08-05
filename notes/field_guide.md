# Agent Field Guide

Read this before changing the repository. Add concise entries when work reveals
a durable constraint, a non-obvious convention, or a recurring failure mode that
would help a future agent. Keep temporary plans and task-specific notes out.

## Who depends on this, and how

- Ordnung pins Entl by exact Git revision, so a change here reaches it only when
  that revision is bumped deliberately.
- Straitjacket, infact, and cowbird consume Entl through Cargo `path`
  dependencies on a sibling checkout, and their CI checks out this repository's
  **default branch**. A change here can therefore break their builds with
  nothing landing in them. Before merging anything that touches a shared
  surface, consider what it does to those three.
- That is not hypothetical. Adding a diagnostic for a recognized language with
  no parser pack broke straitjacket's suite, because it fired on the
  `straitjacket.toml` that configured the run.

## Parser packs

- Packs live in `parser-packs/`. Only a handful of the registered languages have
  one, and that is expected: a pack is a vendored tree-sitter grammar with a
  pinned revision, sha256, and ABI.
- The pack format is built for programming languages — `unit-node-kinds`,
  `error-handling` with its fallible and optional types, `tests` markers, and
  queries for callables and behaviors. None of that has a meaning in TOML or
  YAML, so a pack for a data language could only be a stub that silences a
  check.
- `parse_repository` reports a language it recognized but could not read only
  when that language's `LanguageRole` expects a pack. Reporting every detected
  language made a README and a config file read as unread source, burying the
  real gaps.

## Layout

- `crates/entl-rust-mir` is excluded from the workspace, needs its own nightly,
  and carries its own `rust-toolchain.toml`. The root toolchain file does not
  affect it.
- It and `tools/verbosity` each carry their own `Cargo.lock`, so they are
  separate dependency surfaces. Anything reasoning about lockfile owners has to
  account for all three.
- `tests/fixtures/**` is ignored through `.ordnung/overrides.toml`. Fixtures are
  inputs to tests, not source this repository ships; without that they read as
  TypeScript projects owing CI tasks and a type layer.

## Fleet

- `.github/dependabot.yml` is fleet-owned and comes from the `conf` repository.
  Editing it here is drift, and the next sync overwrites it.
- CI is one `gate` job: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`. Actions are pinned by
  commit SHA; do not swap one for a tag to make updating easier.
