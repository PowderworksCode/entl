# Entl design

## Purpose

Entl is a Rust library for acquiring, shaping, and moving well-understood kinds
of data. It provides domain types and the difficult adapters around them. Rust
iterators and streams remain the ordinary composition layer; storage formats
such as Arrow, JSON, and SQL belong at the edges.

The first domain is codebase inventory because multiple consumers already
need the same facts at different levels:

- source scanners need an ignore-aware walk, language identity, package
  context, and lazy source content;
- codebase auditors need manifests, package boundaries, workspaces,
  lockfile ownership, and evidence; and
- syntax analysis needs identified files before choosing a tree-sitter grammar.

## Current contract

`entl-codebase` exposes two layers over a local working tree. `walk` produces an
immutable `CodebaseTree` containing file facts, language evidence, scoped
diagnostics, and lazy content access. `inspect` parses manifests and resolves
packages, projects, ecosystems, workspaces, and package scripts on top of that
same walk, producing a `CodebaseInventory`. The absolute
canonical root is retained only to support lazy reads. Every exported fact
below the root uses deterministic, codebase-relative `PathBuf` values.

The inventory separates six concepts:

1. A `FileEntry` is one walked file with size, optional language detection, and
   its nearest owning package of each package kind.
2. A `LanguageDetection` is a decision plus inspectable evidence. Filename,
   extension, and shebang are distinct evidence forms. A colocated
   `LanguageProfile` records detection, shallow syntax, project signals, and
   typed facets describing reusable source surfaces such as structured code,
   style hosting, and component construction. Its optional conventions field
   carries test-layout, inline-test, and required-config defaults.
3. A `Manifest` records a recognized manifest even when parsing fails.
4. A `Package` is a package definition. Package existence does not imply
   membership in any workspace.
5. A `Workspace` is a declared membership relation over packages of one kind.
6. A `Project` aggregates the packages, languages, ecosystems, facets, and
   evidence at one codebase boundary.

Directories can contain both Cargo and Node packages. A file can therefore
have two owners without either ecosystem being treated as primary.

Discovery runs registered handlers in four ordered phases: manifests,
relationships, projects, and enrichment. Downstream crates can add project
languages, facets, evidence, or diagnostics without replacing built-in
discovery. Cargo and Node each own their manifest-phase handler and typed
parsing data;
the central inventory draft only accumulates facts and resolves cross-domain
relationships.

Complete file contents are never loaded during the walk. Shebang recognition
reads at most 512 bytes and only for files unidentified by name or extension.
Consumers explicitly call `read_bytes` or `read_text` when needed.

Traversal pruning is registered domain data rather than a walker-owned list.
Unambiguous dependency/cache names can be excluded directly; ambiguous output
names such as `build` require a relevant marker in their ancestry. The Cargo,
Node, and Python definitions own their respective traversal exclusions.

## GitHub facts

`entl-github` is a separate, optional domain adapter. Its `inspect` function
accepts a completed `CodebaseInventory` and returns a `GithubInventory`; it
does not walk the repository again and does not register a hidden codebase
discovery handler.

`GithubInventory` records every repository-owned workflow path, successfully
parsed workflows, and scoped parse diagnostics. A `Workflow` records triggers
and observed `TaskInvocation` values. Each invocation carries a typed task
kind, registered tool profile, workflow location, package-relative working
directory, applicable languages, artifact outputs, package scopes, and
evidence. Expanding a package script retains its manifest as provenance.

Tool profiles and command-to-task classification live in `entl-codebase`
because those facts apply equally to local scripts, other CI providers, and
GitHub Actions. `entl-github` uses those profiles while extracting workflow
invocations; serialized task facts retain stable IDs at the boundary. The
GitHub crate also owns serializable remote repository, branch, security, and
workflow-run fact types. API transports and policy-driven mutations remain
consumer concerns.

## Error model

Failure to establish a usable codebase root is fatal. Errors below that root
are diagnostics whenever the remaining inventory is still valuable: walk
errors, unavailable metadata, invalid manifests, invalid workspace globs, and
declared member patterns that match no discovered package.

This distinction lets policy consumers report an honest `error` for the
affected scope without losing unrelated codebase facts.

## Boundaries

`entl-codebase` owns reusable codebase facts and profiles:

- tree traversal and ignore semantics;
- language identification, evidence, and shallow lexical facts;
- registered language profiles and ecosystem roles;
- registered factual language facets and typed profile relationships;
- optional language conventions colocated with their owning profiles;
- measured relative verbosity per language and per language pair;
- registered tool profiles and command-to-task classification;
- registered artifact profiles and project-scoped artifact instances;
- manifest parsing;
- package and workspace relationships;
- package-script parsing; and
- lazy access to source bytes.

`entl-github` owns observed GitHub facts:

- workflow-file recognition, parsing, and diagnostics;
- package-script expansion into typed task invocations; and
- typed remote repository, branch, security, and workflow-run facts.

Consumers own judgments:

- which files a rule applies to;
- whether language convention defaults are required or locally overridden;
- acceptable package managers and lockfile policy;
- which automation tasks and triggers policy requires; and
- findings, suppressions, remediation, and reporting.

Language profiles and ecosystem profiles are separate registries. An ecosystem
may imply zero or more languages: Cargo implies Rust and Bun implies
JavaScript, while a polyglot build system such as Bazel need not imply any.
This keeps ecosystems related to languages without forcing them into a
language-owned hierarchy.

Tree-sitter will be an optional transform over codebase files, not part of
the inventory walk. `entl-tree-sitter` discovers versioned parser packs at
runtime and loads their Wasm grammars through Tree-sitter's `WasmStore`. Entl
can therefore describe parser availability without compiling grammar crates
into every consumer. Parser packs retain their grammar digest, ABI, source
language, file selectors, comparison domain, and tokenization metadata as
parse provenance. Selection uses the detected Entl language plus the file path;
this permits grammar variants such as TypeScript and TSX to coexist without a
consumer-owned extension table.

## Next adapters

The next likely steps are:

1. refine package adapters with real consumer fixtures, including target-scoped
   Cargo dependencies and pnpm workspace declarations;
2. introduce a read-only tree interface and make the local working tree its
   first implementation;
3. add a Git commit-tree implementation producing the same inventory;
4. add parse caching keyed by content hash, grammar version, and extractor
   version; and
5. add serialization and database projections only after the Rust domain types
   have settled under consumer use.

Fluessig may eventually generate projections for these types. It should not own
walking, polling, checkpointing, or I/O execution, and Entl should not wait for
that generator before proving its domain contracts.
