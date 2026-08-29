# Entl design

## Purpose

Entl acquires source code and observes what it means. It is the extract and
transform layer that consumers build analysis on: where the code came from, what
is in it, and what a compiler says about a given place in it.

Rust iterators and streams remain the ordinary composition layer; storage formats
such as Arrow, JSON, and SQL belong at the edges.

Entl reports observed facts and evidence. Consumers own policy, findings,
remediation, and presentation. That boundary is why several unrelated consumers
can share one acquisition layer: a scanner, an auditor, and a porting estimator
need the same facts and disagree about what the facts mean.

## Two domains, and why they are one library

Entl answers two questions that look unrelated and share one substrate.

**What is in this codebase.** Files, languages, manifests, packages, workspaces,
projects, and the repository configuration around them. This is inventory, and it
is the older and larger of the two.

**What does this code mean.** Where a call goes, what type a name has, which
fields a container declares. This is semantic observation, and it is younger.

They belong in one library because acquisition is the shared problem. Inventory
decides which files exist and what language each is; observation needs that answer
before it can choose a grammar or a compiler. A consumer almost always needs both.

They are separate crate roots because they have different consumers and different
rates of change. Nothing in `entl-semantics` depends on `entl-codebase`.

## Codebase inventory

`entl-codebase` exposes two layers over a source tree. `walk` produces an
immutable `CodebaseTree` containing file facts, language evidence, scoped
diagnostics, and lazy content access. `inspect` parses manifests and resolves
packages, projects, ecosystems, workspaces, and package scripts on top of that
same walk, producing a `CodebaseInventory`. The absolute canonical root is
retained only to support lazy reads. Every exported fact below the root uses
deterministic, codebase-relative `PathBuf` values.

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

Directories can contain both Cargo and Node packages. A file can therefore have
two owners without either ecosystem being treated as primary.

Discovery runs registered handlers in four ordered phases: manifests,
relationships, projects, and enrichment. Downstream crates can add project
languages, facets, evidence, or diagnostics without replacing built-in discovery.
Cargo and Node each own their manifest-phase handler and typed parsing data; the
central inventory draft only accumulates facts and resolves cross-domain
relationships.

Complete file contents are never loaded during the walk. Shebang recognition
reads at most 512 bytes and only for files unidentified by name or extension.
Consumers explicitly call `read_bytes` or `read_text` when needed.

Traversal pruning is registered domain data rather than a walker-owned list.
Unambiguous dependency and cache names can be excluded directly; ambiguous output
names such as `build` require a relevant marker in their ancestry. The Cargo,
Node, and Python definitions own their respective traversal exclusions.

## Semantic observation

### The problem, and the shape of the answer

Syntax alone cannot say what code means. `use std::fs; fs::read(path)` and
`std::fs::read(path)` are the same call written two ways, and only name
resolution knows it. The authority on that question is the language's own
compiler.

The obvious design is a common intermediate representation that every language
lowers into. Entl does not do this, because compilers do not agree on what they
hold. Some expose a control flow graph, some a typed syntax tree, some neither,
and forcing them into one structure discards whichever facts do not fit.

What every compiler can answer is a question about a place in the source: at
these bytes, what does this name refer to, what type does this expression have,
where does this call go. So the shared thing is **the question, anchored to a
span** — not the structure of the answer. Each observer answers what it can and
reports the rest as unresolved.

This makes spans the join key for the whole system. A parse says a function
occupies bytes 100 to 200. A compiler observation says the call at byte 150
resolves to `std::fs::read`. A coverage run says byte 150 executed under test T.
None of those producers knows about the others, and all of them compose.

### How an observation is acquired

Language tooling divides by what it requires to run, and that division decides
whether it can be data or must be code.

| mechanism | requires | example | form |
|---|---|---|---|
| grammar | nothing; hermetic | tree-sitter-c | wasm pack |
| queries | a grammar | `discards.scm` | pack data |
| toolchain driver | the real compiler, as a subprocess | `tsc`, `zig` | observer pack |
| compiler plugin | linking the compiler's internal API | `rustc_private` | crate |

Only the last requires a crate. `entl-rust-mir` links rustc's private API and
rides a pinned nightly toolchain, so it is compiled in and always will be. Every
other mechanism can be discovered at runtime and verified by digest.

This is deliberate. The language axis is where this library grows without bound,
and growth along that axis must not mean a crate, a build dependency, and a
release surface per language.

### Packs

A pack is a versioned, content-addressed directory discovered at runtime. Its
manifest declares what it is, where it came from, and what it provides.

Parser packs are the mature case. `parser.toml` records `schema`, `id`,
`language`, `version`, upstream `source` and `revision`, `license`, tree-sitter
`abi`, and a `sha256` of the grammar, alongside `[files]` extensions and
`[tokenization]` node kinds. Queries live beside the grammar and compile against
it at load; a query that does not compile fails the load, and naming a query the
pack does not ship is an error rather than an empty result. `queries_sha256`
digests the query set, so two packs sharing a grammar remain distinguishable.

Today the toolchain drivers are not packs. `entl-ts-observe` is a thin crate
around `providers/typescript/observe.mjs`, located by a path relative to the
source file, with no manifest, no version, and no digest. `entl-zig-air` is a
crate that shells out and writes Parquet. Both are observer packs that have not
been named as such, and neither carries the provenance the parser packs do.

*Intended:* observer packs use the same mechanism. An `observer.toml` carries the
same identity, versioning, licensing and digest fields, plus the host toolchain it
requires, the observation kinds it emits, and its behaviour when that toolchain is
absent. Discovery, verification and version pinning are shared; a pack's kind and
payload are what differ.

### Where wasm stops

Wasm is the right form for hermetic, deterministic work: grammars, queries, and
plausibly normalization rules. It cannot drive a real toolchain — a wasm module
cannot usefully invoke `tsc` or `rustc`. Observer packs are therefore processes,
not modules. Stating the boundary keeps each new language from relitigating it.

## Shared shapes

Three things are common to every fact Entl produces.

**Span.** The join key. An observation that cannot be anchored to a byte range
cannot compose with the others, and should be reconsidered before it is added.

**Provenance.** Which pack, which version, which digest produced this. Parses
already carry `ParseProvenance` including `queries_sha256`, and every
`InputEvidence` construction site carries it through. This is what makes a cache
safe and a result reproducible, and it should hold for every observation rather
than only for parses.

**Fidelity.** Whether the fact is exactly what the source says. A missing fact and
a quietly wrong one are different failures, and only the second is dangerous.

`ParsedFile::rewrites_narrowed` is the first instance. When a grammar cannot read
a construct, Entl retries the file against a per-language rewrite table; every
rewrite preserves byte length, so a reported span stays correct either way. But a
rewrite that had to choose between two comptime-conditional types produces a
signature narrower than the one in the file, and a consumer that quotes source
rather than merely locating it must not present that as the author's text.

*Intended:* generalize this to a small enum every observer can return — exact,
degraded when a fallback observer was used, narrowed when a rewrite chose, and
unresolved when the question was asked and not answered. A confident empty answer
is the worst result this library can produce, and fidelity is how an observer
declines to give one.

## Repository and forge facts

`entl-github` is a separate, optional domain adapter. Its `inspect` accepts a
completed `CodebaseInventory` and returns a `GithubInventory`; it does not walk
the repository again and does not register a hidden codebase discovery handler.

`GithubInventory` records every repository-owned workflow path, successfully
parsed workflows, and scoped parse diagnostics. A `Workflow` records triggers and
observed `TaskInvocation` values. Each invocation carries a typed task kind,
registered tool profile, workflow location, package-relative working directory,
applicable languages, artifact outputs, package scopes, and evidence. Expanding a
package script retains its manifest as provenance.

Tool profiles and command-to-task classification live in `entl-codebase` because
those facts apply equally to local scripts, other CI providers, and GitHub
Actions. `entl-github` uses those profiles while extracting workflow invocations;
serialized task facts retain stable IDs at the boundary. The GitHub crate also
owns serializable remote repository, branch, security, and workflow-run fact
types. API transports and policy-driven mutations remain consumer concerns.

*Known conflation:* `entl-github` owns both the vocabulary of repository facts and
the GitHub adapter that produces them. Those are different things, and separating
them is cheap while there is one adapter and expensive once there are three. A
neutral repository vocabulary with per-forge adapters is the intended shape.

*Intended:* a source is not always a working tree. The same inventory should be
producible from a local directory, a git commit tree, or a forge API, without the
consumer caring which. That interface does not exist yet.

## Error model

Failure to establish a usable codebase root is fatal. Errors below that root are
diagnostics whenever the remaining inventory is still valuable: walk errors,
unavailable metadata, invalid manifests, invalid workspace globs, and declared
member patterns that match no discovered package.

This distinction lets policy consumers report an honest `error` for the affected
scope without losing unrelated codebase facts.

The same principle governs observation. An absent compiler, an unparsable file,
or an unresolved name is a diagnostic carried alongside the facts that survived,
never a silent empty result.

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

`entl-tree-sitter` owns grammar acquisition, parse execution, and parse
provenance. Consumers own interpretation of the resulting concrete syntax trees.

`entl-semantics` owns the observation vocabulary: what may be asked about a span,
and how an answer is shaped. It owns no language.

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
role is not a language, and a language is not an ecosystem.

Entl does not add a CLI, database, language binding, or generic pipeline
abstraction without a concrete consumer that needs it.

## Direction

Stated as intent rather than schedule. Each item is independently useful and none
blocks the others.

1. **Give the toolchain drivers the pack treatment.** Observer packs with
   manifests, versions, digests and declared toolchain requirements, so that
   adding a language means adding data rather than a crate.
2. **Make every observer speak `entl-semantics`.** `entl-rust-mir` and
   `entl-ts-observe` do. `entl-zig-observe` reports through tree-sitter and
   `entl-zig-air` reports through Parquet, so the shared vocabulary is bypassed
   by half the observers that should be proving it generalizes.
3. **Close the gap between parsing and observation.** Parser packs cover seven
   languages; semantic observation covers three. That ratio decides how much of
   this library's value is actually reachable.
4. **Separate the repository vocabulary from the GitHub adapter**, before a
   second forge makes the split expensive.
5. **Introduce the read-only tree interface**, with the local working tree as its
   first implementation and a git commit tree as its second.
6. **Add parse caching** keyed by content hash, grammar version, and extractor
   version, for which the provenance work above is the precondition.
7. **Refine package adapters** with real consumer fixtures, including
   target-scoped Cargo dependencies and pnpm workspace declarations.
8. **Add serialization and database projections** only after the domain types have
   settled under consumer use.
