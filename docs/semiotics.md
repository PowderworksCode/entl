# Semiotics

*A design sketch. Nothing here is implemented. Where a claim was tested, the
probe and its numbers are given; where it was not, it is marked unverified.*

Semiotics is where language knowledge lives: what a language is, how to
recognise it, what its tooling can be made to say about a program, and how
honestly it said it. It is the destination for everything language-shaped that
Entl is going to shed and that treebank does not already own.

Two names, two things. **Semiotics is the crate. A typebank is what it
produces** — the content-addressed set of span-anchored observations a run
leaves on disk.

This document follows Entl's convention: *intended* marks something proposed
rather than existing.

## Three repositories, three questions

| | question | owns |
|---|---|---|
| **entl** | where did this code come from, and what is in it? | walk and ignore semantics, manifests, packages, workspaces, projects, ecosystems, tool and artifact profiles, forge facts, lazy reads |
| **treebank** | what does the grammar say? | corpora, ranking, fetching, grammar patches, gap ledgers, `parser.toml`, and the parse runtime |
| **semiotics** | what does this language mean, and what does this program mean? | language identity, detection, profiles, facets, conventions, verbosity, the observation vocabulary, and the observers |

The dependency direction follows from that, and it is the opposite of what a
first reading suggests. Semiotics is not a layer on top of acquisition. It is
underneath everything, because language identity is what Entl's walk and
treebank's packs are both written in terms of.

```
                        semiotics
              LanguageId · profiles · detection · conventions
              verbosity · Span · Provenance · Fidelity · Environment
              Definition · Reference · CallEdge · TypeAt · Implements
                    ↑                              ↑
                  entl                         treebank
        acquisition, inventory        grammars, packs, parse runtime,
                    ↑                  corpora, gap ledgers
                    │                              ↑
                    └──────────┬───────────────────┘
                               │
                    cowbird · infact · straitjacket
```

**Semiotics depends on nothing else in the fleet.** That invariant is what makes
the split worth doing; everything below is in service of keeping it true.

## Shape

### The repository

```
semiotics/
  crates/
    semiotics/            identity, detection, profiles, facets, conventions,
                          verbosity, and the observation vocabulary
    semiotics-observe/    the driver, and the observer.toml pack mechanism
    semiotics-store/      content-addressed blobs, Arrow sidecars
    semiotics-c/          libclang; the only tier-1 compiler observer
    semiotics-rust-mir/   rustc_private; excluded from the workspace
    semiotics-zig/        ← entl-zig-observe, intended to dissolve into pack data
    semiotics-cli/
  observer-packs/
    typescript/           observer.toml + observe.mjs   ← providers/typescript
    zig-air/              observer.toml, drives a forked zig
    csharp/  java/        observer.toml, not yet written
  docs/
```

**One crate everyone depends on, two that nobody does.** `semiotics` carries
roughly 2,800 lines — the language third of `entl-codebase` plus
`entl-semantics` — and needs `serde`, `registry-inventory` and `thiserror` and
nothing heavier. No `globset`, no `ignore`, no `toml`, no `parquet`. A consumer
asking "is this file Rust?" links a data crate.

`semiotics-c` and `semiotics-rust-mir` are crates because they link a compiler's
internal API. That is the only case that forces one, and Entl's mechanism table
already says so:

| mechanism | requires | example | form |
|---|---|---|---|
| grammar | nothing; hermetic | tree-sitter-c | treebank pack |
| queries | a grammar | `discards.scm` | pack data |
| toolchain driver | the real compiler, as a subprocess | `tsc`, `zig` | observer pack |
| compiler plugin | linking the compiler's internal API | `rustc_private`, libclang | crate |

Everything else about a language is data. Adding TypeScript, C# or Java means a
manifest and a script, not a crate, a build dependency and a release surface.
That is the property the Entl design doc cares most about on the language axis,
and semiotics inherits it rather than reinventing it.

### The layering

```
layer 0   semiotics                 no fleet dependencies
             ↓
layer 1   entl-codebase        treebank-parse       (both name languages)
             ↓                       ↓
layer 2   semiotics-observe    semiotics-store
             ↓
layer 3   semiotics-c   semiotics-rust-mir   observer-packs/*  (data)
             ↓
layer 4   entl-observe         semiotics-cli
             ↓
consumers cowbird   infact   treebank-cli ──→ semiotics-c (verdict-only)
```

`treebank-cli` is the only arrow that skips the stack. It wants one bit, and
routing it through the driver would make its 850,000-file sweep pay for
machinery it does not use.

`entl-observe` sits in **Entl**, not semiotics: it is the ~30-line adapter that
turns a `CodebaseInventory` into observation units. Putting it on the Entl side
is what keeps semiotics ignorant that inventory exists.

### What a run is

```sh
$ semiotics observe ~/src/bun --out .typebank
```

1. **Entl** walks and inspects, producing projects, languages and packages.
2. **`entl-observe`** turns that into `Vec<ObservationUnit>`, filling
   `project_root` wherever `inspect` resolved one. This is the only line where
   Entl and semiotics touch.
3. **`semiotics-observe`** groups units by `(language, project_root)`. The
   grouping *is* the tier decision: a group with a project root can run tier 2,
   one without it runs tier 1 or nothing.
4. It computes each group's digest, checks the store, and skips what is current.
5. Observers run in parallel. Each returns a `SemanticObservations` carrying its
   own `Coverage` and `Environment`.
6. **`semiotics-store`** canonicalizes, writes the blob, appends the index line.

### What lands on disk

```
.typebank/
  observations/
    sha256-ab12ef….json        envelope and closed-core facts
    sha256-ab12ef….parquet     optional, for million-row payloads such as AIR
  index.jsonl                  {path, provider, digest, byte_range}
```

### What a consumer does

```rust
let bank = semiotics_store::Bank::open(".typebank")?;
for edge in bank.calls_at("src/js_parser.c", 4180..4210)? {
    // edge.to, edge.dispatch, edge.fidelity, edge.provenance.toolchain
}
```

That is the whole surface. No query language, no server, no daemon.

### What it is not

Not a database — infact already runs DBSP for incremental relations, and
semiotics feeds it. Not an LSP — an LSP answers about an editor's live buffer,
semiotics answers about a committed tree. Not a decider — an observation says a
call resolves to `std::fs::read`, never that this matters. Findings,
suppressions and remediation stay with consumers, which is the boundary that
lets a scanner, an auditor and a porting estimator share one producer while
disagreeing about everything else.

## The premise, corrected

The idea that motivates the observation layer is that treebank already runs
compiler front ends and keeps one bit, so the discarded remainder is nearly free.

**That is half wrong, and the wrong half decides the design.**

treebank does not run full front ends. It runs the *weakest configuration each
language admits*, deliberately:

| language | `Lang::validate()` runs | semantic phase |
|---|---|---|
| Rust | `syn::parse_file` — a parser crate; rustc is never invoked | none exists |
| C# | `CSharpSyntaxTree.ParseText`, "parse-only, so unresolved types are not errors" | not run |
| Java | `JavacTask.parse()` — the parse phase, stopping before attribution | not run |
| TypeScript | `ts.createSourceFile` parse diagnostics — no program, no checker | not run |
| C | libclang, parse-only, semantic diagnostics discarded **by rule** | run, then thrown away |

Four of the five have no remainder to keep. Measured, on 200 interlinked files
per language on this machine:

| | parse only | project-scoped semantics | per-file semantics |
|---|---:|---:|---:|
| **TypeScript** 5.9.3 | 54 ms · 0 resolvable | 866 ms · **16×** · 1600 typed | 36,837 ms · **687×** · 1600 typed |
| **C#** Roslyn 4.8 / SDK 8.0.129 | 30 ms · 0 resolved | 557 ms · **19×** · 1200 resolved | 2,364 ms · **79×** · **1000** resolved, 200 errors |
| **Java** javac 21.0.11 | 129 ms · **0 resolved** | 339 ms · **2.6×** · 1200 resolved | 4,364 ms · **34×** · 1200 resolved, 200 errors |

Three things follow.

**Parse-only carries no semantics at all.** Not "less" — none. `JavacTask.parse()`
gives 1,200 method invocations of which `Trees.getElement` resolves zero, because
attribution never ran. Roslyn's parse tree resolves zero without a
`CSharpCompilation` behind it. There is nothing to salvage at the process
boundary because nothing was computed.

**Semantics is cheap only if the unit changes from file to project.** The
whole-project column is 2.6× to 19× the validity bit, which is affordable. The
per-file column is 34× to 687×, and worse than slow: C# per-file resolves only
1,000 of 1,200 invocations, silently losing every cross-file call. treebank's
unit is the file, because it sweeps corpora that do not build — 860,590 `.cs`
files from monorepo checkouts, Debian source tarballs, npm tarballs. The
observation layer's unit is the project, because that is the only unit where a
checker is both correct and affordable. **They cannot share an invocation.**

**C is the exception, and it is the trap.** libclang's parse-only translation
unit — the identical invocation and flags `tools/c-oracle` already runs — carries
resolved callees, USRs, types and byte offsets for free. Verified:

```
good.c, header on -I:      call @6:29 (byte 150) -> mylib_len  type=mylib_size_t  usr=c:@F@mylib_len
same file, header absent:  call @3:29 (byte 120) -> mylib_len  type=int           usr=c:@F@mylib_len
```

Same USR, wrong type. The oracle's *validity* verdict degrades correctly — a
parse error appears, so `ORACLE.md`'s categorical rule says indeterminate. The
*semantic payload* does not degrade. It substitutes `int` and says nothing.
Validity and semantics degrade independently, which is the single most important
constraint in this document.

So semiotics does not inherit treebank's compiler runs. **It inherits treebank's
adjudication discipline**: the three-valued verdict, categories rather than
message text, `other` as a named tripwire, "never invent invalidity", the
negative corpus, and a ledger that states what was not adjudicated. That
discipline is a more developed `Fidelity` than Entl's `Fidelity`, which does not
exist in code.

## Disposition

For each Entl crate, where it goes.

| crate | disposition |
|---|---|
| `entl-codebase` | **splits**: the language third → semiotics; walk, manifests, packages, ecosystems, tools stay in Entl |
| `entl-github` | **stays in Entl**, unchanged |
| `entl-semantics` | **merges into `semiotics`** as the observation vocabulary |
| `entl-tree-sitter` | **→ treebank, entirely**; `repository.rs` deleted |
| `entl-rust-mir` | **→ `semiotics-rust-mir`**, intact, still outside the workspace |
| `entl-ts-observe` | **→ observer pack `typescript/`**; the crate dissolves |
| `entl-zig-air` | **splits**: reader → observer pack `zig-air/`; `store.rs` → `semiotics-store` |
| `entl-zig-observe` | **→ `semiotics-zig`**, intact for now |

### Splitting `entl-codebase`

This is the aggressive reading of "all language things", and it is a third of
Entl's largest crate, so it was worth checking whether the cut is real before
proposing it. It is. Measured:

| | lines | reaches across the seam? |
|---|---:|---|
| **→ semiotics**: `profiles/language.rs`, `profiles/languages/*`, `facet`, `facets`, `convention`, `verbosity` | **2,225** | **no** |
| **stays in Entl**: `ecosystem`, `ecosystems/*`, `tool`, `tools/*`, `artifact`, `artifacts`, `traversal` | 1,142 | — |

The language side references nothing on the Entl side: no ecosystem profile, no
tool profile, no walker, no manifest parsing. The single exception is
`model/id.rs`, 44 lines, which defines language and package identifiers together
and needs splitting. `profiles/verbosity.rs` alone is 1,334 lines of pure
language fact — measured relative verbosity per language and per language pair —
and is the least ambiguous case in the fleet. The `docs/verbosity-*.md` measurement
write-ups and `tools/verbosity` travel with it.

Entl's design doc already drew this line; semiotics makes it structural:

> "Language profiles and ecosystem profiles are separate registries. An
> ecosystem role is not a language, and a language is not an ecosystem."

What Entl keeps is still a coherent crate — tree traversal and ignore semantics,
manifest parsing, packages, workspaces, projects, ecosystem roles, tool and
artifact profiles, traversal pruning, and lazy access to source bytes. It loses
the ability to answer "what language is this?" on its own and calls semiotics
for it during the walk, which is one call in `walk.rs`.

### `entl-semantics` merges into the root crate

Zero dependencies today, so the move is free. It arrives as the observation
vocabulary of `semiotics` rather than as its own crate, because `LanguageId` and
`Span` are wanted by the same consumers and splitting them buys nothing. Three
amendments, argued later: byte spans, `Fidelity` plus `Environment`, and stable
entity ids.

### `entl-tree-sitter` goes to treebank whole

*Changed from an earlier draft, which split it.* treebank is tree-sitter
focused, so it should own grammars end to end — produce, publish, load, and run.
One place then knows the ABI, the pack format, and the dialect gaps, instead of
the producer knowing half and a consumer knowing the other half.

- `manifest.rs` — the `parser.toml` schema and digest verification. treebank
  writes that file; this only reads it. A format belongs with its producer.
- `catalog.rs`, `runtime.rs` — pack loading, query compilation, the parser
  runtime.
- `dialect.rs` — 1,533 lines of per-language rewrite tables, which exist because
  a grammar cannot read a construct. Grammar gaps are treebank's entire reason to
  exist, and it already ledgers the ones it cannot close. Keeping the rewrite in a
  different repository from the patch series was the duplication risk in the
  earlier draft; this removes it.
- `repository.rs` — **deleted.** These 148 lines are `parse_repository(root,
  catalog)`, a convenience driver that calls `entl_codebase::walk()` and loops.
  They are replaced by the seam below.

treebank depends on semiotics for `LanguageId` and reports `rewrites_narrowed`
as semiotics' `Fidelity`. It does not depend on Entl.

### The observers

`entl-rust-mir` moves intact and stays a crate: it links `rustc_private`, rides a
pinned nightly, and is compiled in and always will be.

`entl-ts-observe` **dissolves**. It is 150 lines of Rust wrapping a 308-line
`.mjs`, with no manifest, no version and no digest. Under the pack mechanism the
Rust disappears entirely and `providers/typescript/observe.mjs` becomes
`observer-packs/typescript/` with an `observer.toml` beside it. This is the crate
whose disappearance proves the mechanism.

`entl-zig-air` splits. `air.rs` — 509 lines reading `zig build-obj --verbose-air`
output — is an observer and becomes a pack. `store.rs` — 286 lines of Arrow and
Parquet writing — is **not a Zig fact**; it is a storage backend that happened to
be written inside a Zig crate because Zig was the first observer with millions of
rows. It becomes `semiotics-store`, owned by no language, and is why semiotics
does not need to invent a columnar sink.

`entl-zig-observe` moves intact as `semiotics-zig`. It observes Zig container
fields through tree-sitter rather than a compiler, carrying the type as raw
source text precisely because the grammar mis-groups `*jsc.VirtualMachine` as a
pointer to `jsc`. Observer does not mean compiler, and this crate is the proof.
*Intended:* its 1,349 lines are a query over a parse tree wearing a crate, and
should become treebank pack query data plus a small extractor. Not first.

## The seams

### Semiotics ← Entl and treebank

Both name languages, and that is the whole of it. `entl-codebase` calls
`detect_language` during the walk and stores a `LanguageDetection` on each
`FileEntry`. `treebank-parse` calls `language_profile(&manifest.language)` to
reject a pack naming an unknown language, and `role.expects_parser_pack()` to
decide whether a missing pack is a diagnostic or silence. Neither needs anything
else.

This is the coupling the Entl design doc missed. It argues:

> "They belong in one library because acquisition is the shared problem.
> Inventory decides which files exist and what language each is; observation
> needs that answer before it can choose a grammar or a compiler."

Then, four lines later:

> "Nothing in `entl-semantics` depends on `entl-codebase`."

The file list is a **value**, not a dependency, and so is the project root. The
doc describes a real ordering between layers and mistakes it for a reason to
share a crate. What genuinely is a code dependency is language identity, which
the doc never names — and once that is its own crate at the bottom, the argument
for one library has nothing left to hold.

### Entl → the observation layer

The seam is a value type, not a trait:

```rust
// semiotics-observe (intended)
pub struct ObservationUnit {
    pub path: PathBuf,
    pub language: semiotics::LanguageId,
    pub read: Box<dyn Fn() -> io::Result<Arc<[u8]>> + Send + Sync>,
    /// The build the observer should configure itself from, when there is one.
    /// `None` means per-file observation is the only thing available.
    pub project_root: Option<PathBuf>,
    pub toolchain: Option<ToolchainId>,
}

pub fn observe(units: impl IntoIterator<Item = ObservationUnit>) -> ObservationSet;
```

That struct is what `parse_repository`'s loop already extracts from a
`CodebaseTree`, plus the project root that `inspect` resolves. As a value it can
be produced by Entl's inventory, by a git commit tree, by treebank's corpus
manifest, or by a directory listing.

`project_root: Option<_>` is the axis of the whole design in one field. Entl's
`inspect` is the only thing that turns a directory into a set of projects, so
Entl is what makes the affordable-and-correct column of the cost table
reachable. treebank's corpus cannot supply it — a Debian source tarball has no
configured build — which is exactly why treebank stays per-file.

`entl-observe` provides `units_from(&CodebaseInventory) -> Vec<ObservationUnit>`
and lives in Entl, so cowbird and infact keep a one-line migration and semiotics
never learns that inventory exists.

### treebank ↔ semiotics

*Intended:* the **oracle interface moves to semiotics; the corpus-sweep policy
stays in treebank.** Semiotics owns "run this language's front end over this file
with this environment, and return both the adjudication and whatever was
observed". treebank calls it and keeps one bit for its gap ledger.

This is cheap now in a way it was not under a separate observation repository:
treebank already depends on semiotics for `LanguageId` and `Fidelity`, so calling
`semiotics-c` is the same arrow rather than a new one.

The cost is real and bounded. treebank's sweep must stay fast over 850,000
files, so the observer needs a **verdict-only mode** that builds the translation
unit and skips the AST walk. The C probe shows the unit is constructed either
way; the walk is the only extra work.

`Lang::rank` / `resolve` / `classify` / `grammar_dirs` / `route` stay in treebank
untouched. That is corpus acquisition for grammar work, not codebase inventory.

### Where the shared shapes live

All of them — `LanguageId`, `Span`, `Provenance`, `Fidelity`, `Environment` —
live in `semiotics`, at the bottom, where both sides can reach them without
either reaching for the other. This is the question that was hardest to answer
while the observation layer sat on top of Entl, and it dissolves once semiotics
is the base.

Entl produces no span-anchored fact of its own after the shed:
`ParseDiagnostic` is `(path, message)` and `entl-github` records workflow
locations rather than byte ranges. It takes `LanguageId` and nothing more. If
Entl later wants provenance on inventory facts it will want a *different*
provenance — source tree identity, not toolchain identity — and can define it
without disturbing this.

## A common format is an envelope

**Envelope.** Not an IR. Entl's design already settles this and settles it
correctly; the job here is to say why the evidence supports it, and to correct
one thing.

### Against an IR

Each producer holds something the others cannot express, and there are four
instances in hand rather than an argument from principle:

- **clang** hands over a USR and a resolved type at a byte offset. Verified.
- **tsc** hands over a structural type with no constructor name at all —
  `observe.mjs` is already forced to emit `head: "(anonymous)"` for it.
- **rustc MIR** hands over monomorphized instances, which a source-level schema
  has no slot for. `Dispatch::Unmonomorphized` exists as a warning rather than a
  fact precisely because the mismatch could not be normalised away.
- **`entl-zig-observe`** carries the type as raw source text, because the
  grammar's structural reading of `*jsc.VirtualMachine` is wrong and the author's
  text is the only trustworthy answer.

Any single IR either drops the last one or grows a `raw_text` escape hatch, at
which point it is an envelope with extra ceremony.

### Against a pure envelope

A pure envelope pays nobody, and this is checkable rather than rhetorical:
**neither named consumer depends on `entl-semantics`.** cowbird and infact both
depend on `entl-codebase` and `entl-tree-sitter` and nothing else.

cowbird has instead built its own semantic layer inline — `lsp.rs` drives zls to
resolve import edges, `cgraph.rs` approximates the C linker's symbol graph from
syntax and scores itself against `nm`, `typemap.rs` reports 6.9% `unknown-name`
residue it describes as "gaps in the index, and C FFI types". Those are semiotics
queries answered badly, per-language, in a consumer.

What would pay cowbird is the **closed core**: the same question answered about
Zig and about Rust. A bag of producer-shaped payloads gives it nothing it cannot
already get from tree-sitter.

### So: envelope, with a closed core

- **The envelope** is `Span` + `Provenance` + `Fidelity` + `Environment`. Every
  fact carries it. It is what makes caching safe, staleness detectable, and joins
  legal.
- **The closed core** is the five question kinds `entl-semantics` already has —
  `Definition`, `Reference`, `CallEdge`, `TypeAt`, `Implements` — plus `Coverage`
  saying which were attempted. This is *not* an IR: it normalises the
  **questions**, not the answers. `CallEdge::to` is a `Vec` for exactly this
  reason, and the comment on it is the best statement of the principle in the
  codebase.
- **The open tail** is the change. Today it is `Vec<Gap>` and a `Gap` is a
  string. *Intended:* a producer-keyed typed payload, so `ContainerField`'s dotted
  container path, clang's include environment, and MIR's terminator kinds stop
  being unrepresentable and stop being flattened into prose.

## Scope per language

Which unit each language's tooling actually works on. This is the distinction
that decides the design.

| language | mechanism | works per file? | works per project | native stable entity id |
|---|---|---|---|---|
| **C** | libclang parse-only TU | **yes**, degrading with the include environment | better, with `compile_commands.json` | USR (`c:@F@mylib_len`) — verified |
| **Java** | `JavacTask.analyze()` | technically; 34× and errors on every cross-file reference | yes, 2.6× over parse | binary name via `Elements` — unverified |
| **C#** | `CSharpCompilation` + `GetSemanticModel` | **no** — 79×, and silently loses 200 of 1200 calls | yes, 19× over parse | `GetDocumentationCommentId()` — verified |
| **TypeScript** | `ts.createProgram` + checker | **no** — 687×, reloads `lib.d.ts` per file | yes, 16× over parse, needs `tsconfig.json` | **none** — see below |
| **Rust** | `semiotics-rust-mir` as a drop-in `RUSTC` | **no** — there is no per-file mode at all | requires a working `cargo build` | rustc def path |
| **Zig** | `--verbose-air` from a forked toolchain | unverified | requires a build | unverified |

Reading:

- **C is the only language where per-file observation is first class**, and even
  there the verdict is relative to the include environment, which `ORACLE.md`
  already says out loud.
- **Rust has no cheap tier.** treebank's oracle is `syn`, a parser crate with no
  resolver, so there is nothing to extend — Rust semantics means running a
  different program under a real build. The 16×-to-19× framing does not apply;
  the cost is "a build succeeds or you get nothing".
- **Java's 2.6× is the cheapest semantic tier in the fleet.** Attribution over an
  already-parsed unit is close to free. If any language nearly vindicates the
  original premise, it is Java, and only when the unit is the whole compilation.
- **TypeScript mints no stable entity id.** `observe.mjs` falls back to
  `path:line:column` for local declarations, which changes when a blank line is
  inserted above. That cannot be a cache key or a cross-version join key. C and C#
  both hand over stable ids natively (verified). *Intended:* an `EntityId` must be
  stable under whitespace change — use the language's native id where one exists,
  and a structural path (`module::container::name#overload-index` ) where none does.

Practical consequence: **two tiers, and an observation must say which it came
from.**

- **Tier 1, per file, no build.** C via libclang; every tree-sitter-based
  observer. Cheap, sweepable over corpora that do not build, and the only tier
  treebank can use.
- **Tier 2, per project, build required.** Rust, TypeScript, C#, Java, Zig.
  2.6×–19× the validity bit, correct, and unavailable for most of any corpus.

## Fidelity and loss

Entl's *intended* `Fidelity` — exact / degraded / narrowed / unresolved — is the
right instinct and is not sufficient. The C probe shows why: a call whose header
was missing came back with a **correct USR and the wrong type**. Nothing in the
observation distinguishes it from the good case. There is no fidelity level the
observer could honestly have assigned, because the observer did not know.

So loss is recorded in two places, not one.

**`Fidelity`, per observation** — what the producer knows about this fact:

```rust
pub enum Fidelity {
    /// The fact is what the source says.
    Exact,
    /// A rewrite chose between readings (today's `rewrites_narrowed`).
    Narrowed,
    /// A weaker observer answered — a grammar where a checker was wanted.
    Degraded,
    /// Answered for part of the unit only.
    Partial,
    /// Asked, and not answered.
    Unresolved,
}
```

**`Environment`, per unit** — what the producer *asked for and did not get*:

```rust
pub struct Environment {
    /// Inputs the observer resolved, and inputs it wanted and could not find:
    /// includes, imports, classpath entries, node_modules, sysroot.
    pub inputs_resolved: u32,
    pub inputs_missing: Vec<String>,
    /// The producer's own error counts under its own category names —
    /// clang's `Parse Issue` / `Semantic Issue` / `Lexical or Preprocessor
    /// Issue` / `User-Defined Issue`, and their equivalents. Counts, never
    /// message text: `ORACLE.md` shows classifying prose does not work.
    pub diagnostics: BTreeMap<String, u32>,
    /// The configuration observed: preprocessor symbols, cfg flags, target
    /// triple, tsconfig path. Two configurations of one file are two
    /// observation sets, not conflicting facts about one.
    pub configuration: BTreeMap<String, String>,
}
```

And one invariant, checkable rather than aspirational:

> **An observation from a unit whose `Environment` reports missing inputs may not
> be presented as `Exact`.**

That single rule catches the clang trap, which no per-observation field can. It
is directly inherited from `ORACLE.md`'s categorical rule — "any semantic or
preprocessor error at all demotes a genuine syntax error to indeterminate … it
never invents invalidity" — generalised from a validity verdict to a fact stream.

The three degradation modes the probes found are all different, which is why one
field cannot carry them:

| producer | when context is missing | mode |
|---|---|---|
| clang | resolves the call, reports the wrong type | **confidently wrong** — dangerous |
| Roslyn | drops 200 of 1,200 calls, reports 200 errors | **silently incomplete** |
| javac | resolves everything resolvable, reports 200 errors | **honest and noisy** |

`Coverage` handles the second. `Environment` handles the first. Only the third is
already safe today.

**One correction to `Coverage` as it stands.** `SemanticObservations::merge` ANDs
coverage across units, so one unbuildable package erases a true coverage claim
about ninety-nine good ones. The comment defends this as conservative and it is,
but it is conservative in a way that destroys information. *Intended:* a merged
set keeps coverage **per unit** and derives the conservative answer on demand.

## Versioning and staleness

`Provenance` already records `provider`, `provider_version`, `toolchain` and
`unit`, which is most of the answer, and `entl-rust-mir` already stamps
`nightly-2026-07-18` — exactly right for a representation whose instability is
explicit.

Three fields are missing, and each corresponds to a way an observation goes stale
without any of the four changing:

1. **`inputs_digest`** — content hash of the bytes observed. Without it,
   staleness is a filesystem timestamp question.
2. **`pack_digest`** — what `queries_sha256` already does for parser packs, so
   two observer packs wrapping the same toolchain stay distinguishable.
3. **`resolution_digest`** — hash of the resolved dependency set (`Cargo.lock`,
   `package-lock.json`, `compile_commands.json`, the classpath). **A `cargo
   update` changes MIR without changing one byte of source in the unit.** This is
   the field whose absence would bite hardest and it is the least obvious.

With those, staleness is a comparison rather than a heuristic:

| mismatch | meaning |
|---|---|
| `inputs_digest` | the source changed — re-observe |
| `resolution_digest` | dependencies moved — re-observe |
| `toolchain` | a different compiler answered — re-observe, and the old facts may still be worth diffing |
| `provider_version` | the extractor changed — re-observe |
| `schema` | the consumer must decide; a bump is a migration |

The MIR question specifically: a consumer knows a MIR observation is stale
because the `toolchain` string does not match the toolchain now installed. It
cannot know whether MIR's *shape* changed underneath — that is what
`provider_version` plus a pinned toolchain is for, and it is why
`semiotics-rust-mir` carries its own `rust-toolchain.toml` and always will.

## Joining with tree-sitter

Spans are the join key, and today they do not join. Two problems: one verified
defect and one genuine disagreement between tools.

### `Span` must carry bytes

`entl_semantics::Span` is `path` plus line and column. The design doc says "bytes
100 to 200"; the code says line 4 column 7. Worse, `observe.mjs` derives its
columns from TypeScript's UTF-16 offsets. Verified, on
`const s = "café 🎉"; export const n = s.length;`:

| producer | reports |
|---|---|
| `entl-ts-observe` | column **39** (UTF-16 code units) |
| tree-sitter, clang | byte **41** / column **42** |

Three units of skew from two non-ASCII characters, silently, on the join key for
the whole system. *Intended:* `Span` carries `start_byte` / `end_byte` as the
identity, with line and column alongside for display only. Every producer in the
fleet can supply bytes — `clang_getFileLocation` returns one, tree-sitter is
byte-native, and TypeScript's UTF-16 offsets convert exactly given the source.

### When the two tools disagree about what the file contains

`treebank-csharp/LOCAL-PATCHES.md` is the worked example. Roslyn parses the
**active configuration**: with no symbols defined, `#if FOO` is false, the other
branch is disabled text, and nothing in it can make the file invalid. tree-sitter
parses **all** branches into one tree. For

```csharp
#if BUILD_ENGINE
namespace Microsoft.Build.BackEnd.Components.Caching
#else
namespace Microsoft.Build.Shared
#endif
{
```

Roslyn sees one namespace declaration; the grammar sees two and then a `{`. There
is no single well-formed tree, and neither tool is wrong. treebank measured the
consequence: of 7,148 oracle-valid grammar failures, **4,617 are caused only by
conditional compilation** and are not bugs anyone can fix.

Realigning the offsets is the wrong response, because the disagreement is not
about offsets. It is about which text exists.

*Intended:* **the join is refused rather than fudged.**

- `Span` carries a `text_digest` — a hash of the unit as *that producer read it*.
  A producer that read rewritten, reduced or preprocessed text has a different
  digest, and a consumer joining across a digest boundary gets a diagnostic
  rather than a silent mismatch.
- `Environment::configuration` records the preprocessor symbols, `cfg` flags or
  target in effect. Two observations of one file under two configurations are
  **two observation sets**, joinable with each other and not with a parse that
  saw all branches.
- For macro and generated code, *intended, unverified:* `Span` gains
  `expanded_from: Option<Box<Span>>`. clang exposes spelling location and
  expansion location separately (`clang_getSpellingLocation` /
  `clang_getExpansionLocation`), which is what would let a fact about generated
  text join back to the macro the author wrote. Not probed.

This costs consumers something honest: a query spanning a preprocessor boundary
returns a gap instead of an answer. That is the correct outcome. cowbird's
`cgraph.rs` already lists "conditional-compilation branches the canonical parse
did not see" among the things it knowingly misses; semiotics should make that
visible in the data rather than in a module comment.

## Consumers

A format with no named consumer is speculation. Both existing consumers have
queries they cannot answer today, and each has already built a worse version of
this inside itself.

### cowbird `src/analysis/cgraph.rs`

Today it reconstructs the C dependency graph — "which file defines the symbols
this file references" — from syntax: a definition index of non-`static` functions
and file-scope variables, plus identifiers a file uses but does not bind. It is
scored against `nm` over actually-built objects, with an acceptance bar of ≥95%
precision and ≥90% recall, and it names three things it knowingly misses.

**The query it could not answer before:** *for a symbol introduced by macro
expansion, which translation unit defines it?* `cgraph.rs` lists macro-generated
definitions (`define_commit_slab`) as a known miss — the dialect table blanks
their sites so files parse, and the generated names resolve nowhere and land in
`unresolved`. libclang resolves them, because it observes after expansion.
Verified that the mechanism exists (USR and resolved cursor from the parse-only
TU); not verified against cowbird's corpus.

This is the strongest case in the fleet: the tier-1, per-file, no-build C
observer is exactly what cgraph needs, and it is the one language where the cost
is genuinely near zero.

### cowbird `src/analysis/typemap.rs`

Measured over Bun's 61,538 signature type positions, the rewriter resolves ~70%.
The residue includes 6.9% `unknown-name` — "gaps in the index, and C FFI types".

**The query:** *what does this name resolve to, when the index does not have it?*
A Zig observer answers the index gaps directly. The C FFI half is answered by the
C observer, across a language boundary the current design cannot cross at all.
Note what it does **not** answer: 14.6% `pointer-needs-class` is ownership, which
`typemap.rs` correctly says "is not a property of the type at all". Semiotics
must not pretend to answer it.

### cowbird `src/analysis/lsp.rs`

A hand-rolled LSP client driving zls, with its own `Outcome` enum —
`Resolved` / `Timeout` / `Empty` / `BadUri` / `NoResult` — so a failure can be
attributed instead of lumped under "zls is slow".

That is an observer pack with a fidelity enum, written in a consumer because
there was nowhere else to put it. It becomes `observer-packs/zls/`, and its
`Outcome` maps onto `Fidelity` almost term for term. This is the clearest
evidence that the shape is right: someone already built it, in the wrong place,
and arrived at the same abstractions.

### infact

Finds repeated tree-sitter token sequences; near clones preserve syntax and
identifier/literal equality patterns under consistent substitution.

**The query:** *are these two token-identical fragments the same code, or do they
merely spell the same names?* Two fragments calling `.new()` are a clone only if
both resolve to the same definition. Today infact cannot tell `Vec::new` from
`MyVec::new`, so it cannot suppress a false pair or promote a true one that
substitution obscured. A `Reference` observation over the same spans answers it,
and DBSP is already the right machinery to maintain it incrementally.

### treebank

Becomes a consumer of the verdict-only mode. It gets the same adjudication it has
today with one fewer copy of the oracle to maintain.

## Storage and shape on disk

A typebank is **content-addressed blobs plus a small index**. Not a database.

**The digest is the cache key and the staleness check in one:**

```
digest = sha256(inputs_digest ‖ resolution_digest ‖ provider ‖ provider_version
                ‖ toolchain ‖ pack_digest ‖ canonical(configuration) ‖ schema)
```

Every input to that hash is a field the previous two sections argued must exist
anyway. A consumer that can compute the key knows without reading anything
whether its cached answer is still good, which is the property Entl's provenance
work was a precondition for.

`SemanticObservations::canonicalize()` already sorts and dedups every collection
so that the same source and toolchain produce byte-identical output. That is what
makes content addressing work at all, and it exists today.

**Why not a database as the primary artifact.** It is not diffable, not
content-addressed, and not reviewable. More concretely, infact already runs DBSP
for incremental relations — semiotics should feed that, not compete with it. A
consumer wanting SQL builds a projection; the Entl design doc's Direction item 8
says the same thing about serialization, and for the same reason.

**Why both JSON and Parquet.** `entl-zig-air` already writes Parquet because AIR
instruction counts run to the millions, and that was the right call. The envelope
is small and belongs in reviewable JSON; a payload with millions of rows belongs
in a columnar file. One digest names both, so the pair is atomic. This is what
`semiotics-store` is for, and it is why `store.rs` should be lifted rather than
deleted.

**Why it is cheap to query.** Facts sorted by `(path, start_byte)` make a span
range query a binary search, and `canonicalize()` is already half of that sort.
The index carries each blob's path set and byte range, so "what does anything
know about `src/lib.rs:150`" reads one index line and one blob rather than
scanning.

**Why it is safe to cache.** The digest covers the toolchain and the resolved
dependency set, not just the source. That is the difference between a cache that
is safe and one that is merely fast.

## Where I disagree with Entl's design

Stated directly.

1. **`Span` is line and column, and must be bytes.** Verified skew between
   `entl-ts-observe` and every byte-native producer, on any line containing
   non-ASCII. The design doc's prose is right and the code does not implement it.
2. **`Fidelity` alone is insufficient.** It cannot express "the answer is present
   and wrong", which is the failure the C probe produced and the one the design
   doc itself calls the dangerous kind. `Environment` plus the not-`Exact`
   invariant is the fix.
3. **"Two domains, one library" defends a real relationship in the wrong form.**
   The coupling is not inventory — that is a value. It is language identity, which
   the doc does not mention, and which belongs *below* both domains rather than
   inside one of them.
4. **`EntityId` has no stability requirement**, and `entl-ts-observe` mints
   `path:line:column` for local declarations, which changes when a blank line is
   inserted above it. C and C# hand over stable ids natively (verified). The
   schema should require stability under whitespace change.
5. **`Coverage` merging destroys information.** ANDing across units means one
   unbuildable package erases a true claim about the rest. Keep it per unit.

Where the design is right and should not be relitigated: the rejection of a
common IR; spans as the join key; provenance on every fact; the pack mechanism
and its runtime discovery; the wasm boundary; the separation of language and
ecosystem registries; and treating an absent compiler as a diagnostic rather than
a silent empty result. Those hold up, and semiotics inherits them unchanged.

## Build order

1. **`semiotics`, the crate.** Merge `entl-semantics` with the language third of
   `entl-codebase`. Mechanical, measured clean, and it unblocks everything else.
   Entl gains one dependency and loses 2,225 lines.
2. **`semiotics-c` and `semiotics-store`, consumed by cowbird's `cgraph.rs`.**
   The first slice that matters. C is the one language where the observer is
   nearly free to write, and cowbird already has an `nm`-scored ≥95% / ≥90% bar to
   pass or fail against. If it clears, the design is load-bearing; if not, we
   learn that before anything is built on top.
3. **`entl-tree-sitter` → treebank.** The largest mechanical move, and the one
   that breaks cowbird and infact, so it goes after the seam has been exercised
   once.
4. **`semiotics-observe` and TypeScript as the first observer pack.** The slice
   that proves adding a language is data. `entl-ts-observe` disappears.

`semiotics-rust-mir` moves whenever convenient — it is outside the workspace and
pinned to its own nightly, so it is the least entangled thing in the fleet.

## What was verified

**Run on this machine.**

- TypeScript 5.9.3, 200 interlinked files: 54 ms parse-only / 866 ms
  whole-project checker / 36,837 ms per-file checker.
- C# Roslyn 4.8 via SDK 8.0.129, 200 files: 30 ms / 557 ms / 2,364 ms, with
  per-file resolving 1,000 of 1,200 invocations and reporting 200 errors.
- Java javac 21.0.11, 200 files: 129 ms / 339 ms / 4,364 ms, with parse-only
  resolving **zero** invocations.
- libclang 20.1.2, parse-only with `KeepGoing`: resolved callee, USR, type and
  byte offset present; with the header absent the same call reports the same USR
  and type `int` instead of `mylib_size_t`.
- Roslyn mints stable entity ids via `GetDocumentationCommentId()` — the probe
  returned, for three resolved LINQ calls (a doubled backtick is the generic
  arity marker):

  ```
  M:System.Linq.Enumerable.Where``1(System.Func{P.IShape0,System.Boolean})
  M:System.Linq.Enumerable.FirstOrDefault``1
  M:System.Linq.Enumerable.ToList``1
  ```

- `entl-ts-observe` reports column 39 where tree-sitter and clang report byte 41,
  on a line containing `café 🎉`.
- The `entl-codebase` split line: 2,225 lines on the language side, 1,142 on the
  Entl side, and the language side references nothing across the seam except
  `model/id.rs`. Established by grep over `profiles/` and `model/`, not by
  attempting the split.

**Read, not run.** Every Entl crate's dependency edges; treebank's five
`validate()` implementations and the `add-c-grammar` libclang oracle;
`ORACLE.md`'s gcc-versus-libclang table and its four-category rule;
`LOCAL-PATCHES.md`'s 4,617 inherent / 2,531 actionable split; cowbird's and
infact's dependency sets and the module comments quoted above.

**Not verified.** rustc MIR extraction cost (needs the pinned
`nightly-2026-07-18` toolchain). Zig AIR (no `zig` on this machine). javac's
`Elements` binary names as a stable entity id. clang's spelling-versus-expansion
locations for the macro join. Whether the C observer's edges actually clear
cowbird's ≥95% / ≥90% bar on a real corpus — the single most valuable unverified
claim here, and the one to test next.

## Open questions

1. **Does the C observer clear cowbird's `nm` bar?** Everything in the consumer
   section rests on it. Testable today against the corpus cowbird already scores.
2. **Do `semiotics` and `semiotics-observe` stay separate crates?** Merging them
   would make semiotics literally one crate, at the cost of `toml` and `sha2` on
   every consumer that only wanted the language registry. Kept apart here; it is a
   close call.
3. **Does `LanguageProfile` split as cleanly as the line count suggests?** The
   2,225/1,142 measurement says the modules do not reference each other, but
   `model/id.rs` defines language and package identifiers together and
   `LanguageDetection` is stored on `FileEntry`. That boundary is the one place the
   split could turn out to be more than a move.
4. **Where do dialect rewrite tables ultimately live inside treebank?** Proposed
   as pack data emitted alongside the grammar, rather than a crate-level table, so
   a gap and its workaround are recorded in one place. Not decided.
5. **Is tier 1 worth having for anything but C?** Every tree-sitter observer is
   tier 1, but tree-sitter observations are not compiler observations. If C is the
   only compiler in tier 1, the tier distinction may be one language wearing a
   general name.
