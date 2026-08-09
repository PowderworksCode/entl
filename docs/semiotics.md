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
| **treebank** | can a grammar read this language yet? | corpora, ranking, fetching, sweeps, oracles, grammar patches, gap ledgers — **automation that produces parser packs** |
| **semiotics** | what does this language mean, and what does this program mean? | language identity, detection, profiles, facets, conventions, verbosity, the observation vocabulary, the parse runtime, the observers, anchoring, and the pack schemas |

treebank is a **pipeline, not a library.** Nothing links it. It ranks packages,
fetches corpora, sweeps them against a grammar, adjudicates with an oracle,
carries a patch series, and publishes a parser pack and a gap ledger. The pack
is its product.

That decides the two questions the split kept snagging on. **The parse runtime
belongs to semiotics**, because running a pack is consuming one, and a consumer
is not the producer. And **the interface between treebank and semiotics is data**
— `parser.toml` and a `.wasm`, read from a directory at run time — not a crate.

Which is only entl's own principle, that growth along the language axis must not
mean a crate, applied one level up at the repository boundary.

```
                          semiotics
              LanguageId · profiles · detection · conventions
              verbosity · Span · Provenance · Fidelity · Environment
              Anchor · the five question kinds · parse runtime
              observers · pack schemas
                    ↑                           ↑
                    │                           │
                  entl                      treebank
             acquisition,              automation only; writes packs,
              inventory                links semiotics-c for its oracle
                    ↑                           ┊
                    │                           ┊ parser packs, as DATA
                    └──────────┬────────────────┘
                               │
                    cowbird · infact · straitjacket
```

**Every code arrow points into semiotics.** treebank and semiotics exchange pack
data, not types. entl depends on exactly one thing. There is no cycle at crate
level or at repository level, which is the property the earlier drafts kept
losing.

The cost, stated plainly: **entl is not standalone.** It records what language a
file is, and that vocabulary lives in semiotics. See *Why entl depends on
semiotics* for the alternative and why it was not taken.

## Shape

### The repository

```
semiotics/
  crates/
    semiotics/            identity, detection, profiles, facets, conventions,
                          verbosity, the observation vocabulary, and the
                          pack manifest schemas
    semiotics-parse/      ← entl-tree-sitter: pack loading, parser runtime,
                          dialect rewrites, and anchoring
    semiotics-observe/    the driver, and the observer.toml pack mechanism
    semiotics-store/      content-addressed blobs, Arrow sidecars
    semiotics-c/          libclang; the only tier-1 compiler observer
    semiotics-rust-mir/   rustc_private; excluded from the workspace
    semiotics-zig/        ← entl-zig-observe, intended to dissolve into pack data
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
layer 1   semiotics-parse   semiotics-store   entl-codebase
             ↓                                     ↓
layer 2   semiotics-c   semiotics-rust-mir   semiotics-zig   entl-github
             ↓                                     ↓
layer 3   semiotics-observe                   entl-observe
             ↓                                     ↓
consumers cowbird   infact   treebank-cli ──→ semiotics-c (verdict-only)

                  treebank-cli ┄┄ writes ┄┄→ parser-packs/  ┄┄ read by ┄┄→ semiotics-parse
```

No arrow leaves semiotics. `semiotics-parse` reads a pack directory at run time
and never links the thing that wrote it; `treebank-cli` links `semiotics` for
`LanguageId` and `Fidelity` and `semiotics-c` for its oracle, and nothing links
`treebank-cli`.

`semiotics-cli` is deliberately absent. Observing a tree means walking it and
resolving projects, which is entl's job, so a CLI that did the whole run would
put an entl dependency inside the semiotics repository and reintroduce a cycle
from the other side. The integration point is `entl-observe`, and consumers
drive it — which is already how cowbird and infact consume `entl-codebase`. If a
CLI is wanted later it belongs in **entl**, where the walk is.

`treebank-cli` is the only arrow that skips the stack. It wants one bit, and
routing it through the driver would make its 850,000-file sweep pay for
machinery it does not use.

`entl-observe` sits in **Entl**, not semiotics: it is the ~30-line adapter that
turns a `CodebaseInventory` into observation units. Putting it on the Entl side
is what keeps semiotics ignorant that inventory exists.

### What a run is

```sh
$ entl observe ~/src/bun --out .typebank      # intended; see The layering
```

1. **Entl** walks and inspects, producing projects, languages and packages.
2. **`entl-observe`** turns that into `Vec<ObservationUnit>`, filling
   `project_root` wherever `inspect` resolved one. Entl's other contact with
   semiotics is `LanguageId` during the walk, and nothing else.
3. **`semiotics-observe`** groups units by `(language, project_root)`. The
   grouping *is* the tier decision: a group with a project root can run tier 2,
   one without it runs tier 1 or nothing.
4. It computes each group's digest, checks the store, and skips what is current.
5. Observers run in parallel. Each returns a `SemanticObservations` carrying its
   own `Coverage` and `Environment`.
6. **`semiotics-store`** canonicalizes, writes the blob, appends the index line.
7. Optionally, **`semiotics-parse`** anchors each observation against the parser
   pack for its language and writes an anchor blob beside it. This step is
   re-runnable on its own whenever treebank publishes a new pack.

### What lands on disk

```
.typebank/
  observations/
    sha256-ab12ef….json        envelope and closed-core facts
    sha256-ab12ef….parquet     optional, for million-row payloads such as AIR
  anchors/
    sha256-ab12ef….7c40d1.json observations resolved against one parser pack
  index.jsonl                  {path, provider, digest, byte_range}
```

An anchor blob is named by the observation digest and the pack digest together,
because it is a function of both. A typebank is valid with none, some or all of
its observations anchored — see *Anchoring to the parse tree*.

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
| `entl-tree-sitter` | **→ `semiotics-parse`**, entirely; `repository.rs` deleted |
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

### `entl-tree-sitter` goes to semiotics whole

*Reversed from an earlier draft, which sent it to treebank.* The reversal
follows from treebank being a pipeline rather than a library: **running a pack
is consuming one**, and the consumer is not the producer. Everything in this
crate reads packs, so all of it becomes `semiotics-parse`.

- `catalog.rs`, `runtime.rs` — pack loading, query compilation, the parser
  runtime.
- `dialect.rs` — 1,533 lines of per-language rewrite tables, applied at parse
  time when a grammar cannot read a construct. This is the one that looks like
  treebank's. It is not: treebank's response to a gap is a patch or a ledger
  entry, and a rewrite is what a *reader* does when it must parse anyway.
  *Intended:* the rewrite table ships in the pack as data, so a gap and its
  workaround are recorded together and treebank emits both.
- `manifest.rs` — the `parser.toml` schema and digest verification. **Also
  semiotics**, which reverses what an earlier draft argued.

That last one deserves the correction spelled out. The earlier draft said a
format belongs with its producer, so `parser.toml`'s schema should live in
treebank. That is the right rule when the producer is upstream of the reader.
Here the producer is *downstream*: treebank links semiotics for `LanguageId`
and `Fidelity`, so treebank can write the format using semiotics' own type,
while semiotics reads it without linking treebank. **The reader owns the format
when the writer is downstream** — and the alternative would put a treebank
dependency inside semiotics, which is the cycle this whole arrangement exists
to avoid.

- `repository.rs` — **deleted.** These 148 lines are `parse_repository(root,
  catalog)`, a convenience driver that calls `entl_codebase::walk()` and loops.
  They are replaced by the seam below.

Anchoring then lands where it belongs without argument: `semiotics-parse` has
the runtime, so resolving a span against a node is a local call rather than a
repository boundary.

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
`FileEntry`. `treebank-cli` names the language a pack is for when it writes
`parser.toml`, and reports `rewrites_narrowed` as `Fidelity`. Neither needs
anything else. (`semiotics-parse` reads that same profile to reject a pack
naming an unknown language, but it is in the same repository, so it is not a
seam.)

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

### treebank → semiotics, one way, and a pack going back

treebank links `semiotics` for `LanguageId` and `Fidelity`, and `semiotics-c`
for its oracle. Semiotics links nothing of treebank's. What comes back the other
way is a **parser pack**: a directory containing `parser.toml`, a `.wasm`, and
queries, read at run time and verified by digest.

That asymmetry is the point. A pipeline's output should be an artifact, not an
API, and a pack already is one — versioned, content-addressed, and discoverable
without a build. It is also what lets a pack be published, vendored, or pinned
independently of either repository's release cycle.

*Intended:* the **oracle interface moves to semiotics; the corpus-sweep policy
stays in treebank.** Semiotics owns "run this language's front end over this file
with this environment, and return both the adjudication and whatever was
observed". treebank calls it and keeps one bit for its gap ledger.

The cost is real and bounded. treebank's sweep must stay fast over 850,000
files, so the observer needs a **verdict-only mode** that builds the translation
unit and skips the AST walk. The C probe shows the unit is constructed either
way; the walk is the only extra work.

`Lang::rank` / `resolve` / `classify` / `grammar_dirs` / `route` stay in treebank
untouched. That is corpus acquisition for grammar work, not codebase inventory.

### Why entl depends on semiotics

This is the one seam with no clean answer, and it is worth stating the trade
rather than hiding it.

Entl records what language each file is. Detection needs five of
`LanguageProfile`'s thirteen fields — `extensions`, `source_extensions`,
`filenames`, `shebangs`, `supersedes`. The other eight — `role`, `facets`,
`comments`, `conventions`, `config_files`, `package_dependencies` — are meaning,
and they live in the **same struct literal per language**:

```rust
pub static PROFILE: LanguageProfile = LanguageProfile {
    id: "rust", extensions: &["rs"], filenames: &[], shebangs: &[],   // recognition
    role: Programming, facets: &[&STRUCTURED_CODE],
    comments: Some(&syntax::RUST), conventions: Some(…),              // meaning
};
```

So there are exactly two options and no clever third.

**Entl depends on semiotics.** One registry, one edit to add a language, and
entl is not standalone.

**Entl keeps recognition and semiotics takes meaning.** Entl depends on nothing,
at the price of splitting one profile across two repositories. Every new
language then needs two registrations in two places, and they will drift: a
language added to entl's recognition table and not to semiotics' meaning table
detects correctly and then silently has no comment syntax.

**Taken: the first.** The growth axis — how much a new language costs — is the
axis both design documents care most about, and two registrations is a real tax
on it. What makes this feel like a layering violation is the name: semiotics
sounds like analysis, and acquisition depending on analysis is backwards. But
the crate entl links is the fleet's **language vocabulary**, and the observation
machinery is in other crates in that repository which entl links none of.
Depending on a vocabulary is the same kind of dependency as depending on
`serde`.

If entl being standalone is a hard requirement rather than a preference, the
recognition/meaning split is viable and this section is where to reverse it.

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

## Packs, and the two kinds of them

"Pack" is doing two jobs in this fleet, and conflating them is how the earlier
drafts kept producing awkward manifests.

| | subject | what it is | changes when |
|---|---|---|---|
| **parser pack** — treebank | a *language* | a grammar you **run** | the grammar changes |
| **observer pack** — semiotics | a *language* | a toolchain driver you **run** | the driver changes |
| **infact pack** — `infact-packs/rust-core` | a *package at a version* | facts you **look up** | the library or the compiler changes |

The first two are the same kind of thing: a **capability**, keyed by language,
discovered at run time, verified by digest, and executed. They should share one
manifest, and `observer.toml` is that manifest for the second.

The third is a different kind: **knowledge**, keyed by a package at a version.
It is not run; it is joined against. Forcing it into a capability manifest would
mean pretending a fact about `core 1.93.1` is a tool.

### Knowledge packs already solved versioning

This matters more than the taxonomy, and it is the reason to look at
`infact-packs/rust-core/pack.toml` before writing anything new:

```toml
provides = ["rust.call-effects"]
[subject]    kind = "language"  language = "rust"  ecosystem = "cargo"
             name = "core"  version = "1.93.1"
[[sources]]  kind = "toolchain"  name = "rust"  version = "1.93.1"  sha256 = "…"
[derivation] generator = "infact"  generator-version = "0.0.0"  analyzer-sha256 = "…"
[compatibility.compiler]  name = "rustc"  version = "1.93.1"
[[contents]] path = "…"  kind = "call-effects"  sha256 = "…"
```

That is `Provenance` — `provider`, `provider_version`, `toolchain`, `unit` — plus
the `pack_digest` and per-content digests this document proposed adding, already
designed and already shipping. It also carries two things `Provenance` lacks and
should gain: `provides`/`requires`, which lets a consumer ask what a pack can
answer without loading it, and `[compatibility.compiler]`, which is the staleness
check stated as a constraint rather than as a string comparison.

**So a semiotics observation set for a released library *is* an infact knowledge
pack.** `provides = ["c.call-graph"]`, subject a package at a version, sources
the toolchain, compatibility the compiler. Same shape, same manifest.

*Intended:* adopt `pack.toml` as the knowledge-pack manifest rather than minting
a third vocabulary, and add the two fields it does not need but a working tree
does:

- `inputs_digest` — content hash of the bytes observed. A released library is
  identified by `[subject].version`; a working tree is not identified by
  anything until you hash it.
- `resolution_digest` — hash of the resolved dependency set (`Cargo.lock`,
  `package-lock.json`, `compile_commands.json`, the classpath). **A `cargo
  update` changes MIR without changing one byte of source in the unit.**

Everything else is already there.

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

What is missing is covered by adopting infact's `pack.toml` — see *Packs, and
the two kinds of them*. `[sources]`, `[derivation]` and `[compatibility.compiler]`
carry the digests and the compiler constraint; `inputs_digest` and
`resolution_digest` are the two fields a working tree needs that a released
library does not.

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

### Anchoring to the parse tree

Spans join by arithmetic: a consumer asks what covers bytes 4180 to 4210 and
gets an answer if the ranges overlap. That works, and it leaves the consumer
guessing at extents. The better question is whether a compiler's fact lands on
a *node* — whether the two tools agree about the shape of the thing, not just
its address.

**Measured, and the answer is yes.** Over the first 40 `.c` files of a git
checkout, every call libclang reported was matched against the smallest
tree-sitter node containing its extent:

| | calls | |
|---|---:|---:|
| exact byte match on a `call_expression` node | **8,392** | **99.5%** |
| exact byte match on some *other* node kind | 38 | 0.45% |
| contained but not exact | 1 | 0.01% |
| straddling a node boundary, or no node at all | **0** | 0% |

Two things in that table are worth more than the headline.

**Macros do not break it.** 1,716 of the 8,431 calls (20.4%) are macro
expansions as far as clang is concerned, and 1,678 of those still land exactly
on a `call_expression` — because a function-like macro invocation is spelled
like a call and the grammar reads it as one. The failure mode assumed in the
earlier draft, where macro-generated facts have no node to anchor to, mostly
does not occur.

**The disagreements are informative rather than noisy.** All 38 are macro
expansions, and the archetype is a single one:

```
abspath.c   clang: call to __errno_location()   tree-sitter: identifier `errno`
```

`errno` is a macro that expands to a function call. Byte-for-byte the two tools
agree on the extent exactly; they disagree about what kind of thing is there.
Neither is wrong. This is the cleanest example in the fleet of a fact that
exists in the compiler and has no counterpart in the grammar, and it argues for
recording the node kind rather than a fidelity ladder.

So, *intended:*

```rust
/// A span resolved against a parse tree. Derived, never authored.
pub struct Anchor {
    /// The node the span landed on, and the pack whose grammar named it.
    pub node_kind: String,
    pub pack: ParserPackId,
    pub node_span: Span,
    pub fit: Fit,
}

pub enum Fit {
    /// The observation's extent is exactly the node's extent.
    Exact,
    /// Contained by the node, but narrower.
    Inside,
    /// Crosses a node boundary, so the two tools disagree about structure.
    Straddles,
    /// No node contains it: generated text, or a configuration the grammar
    /// did not read.
    Absent,
}
```

A consumer compares `node_kind` against what it expected — `call_expression`
for a `CallEdge` — and a mismatch is the finding. `Fit` covers the structural
cases; the kind covers the semantic ones, and the measurement says the kind is
where the action is.

**The anchor is derived, optional, and separately versioned.** Three reasons,
and the third is the one that decides it:

1. Requiring an anchor would make a parser pack a prerequisite for observation.
   Entl ships packs for seven languages and none of them is C# or Java, so
   mandatory anchoring would block the two languages whose observers are
   cheapest to write.
2. Bytes remain the identity. Node identity in tree-sitter is per-parse and not
   stable, so any durable node reference is *derived from* byte offsets — which
   means anchoring decorates the span key rather than replacing it.
3. **The two have different lifetimes.** Re-anchoring needs a grammar and a
   file; re-observing needs a working toolchain and a build. treebank publishes
   new packs continuously as it closes grammar gaps, so anchors want to be
   recomputed often against observations that are expensive and rarely
   recomputed. Fusing them into one artifact would mean re-running a compiler to
   pick up a grammar fix — and since a pack arrives as data rather than as a
   dependency bump, that recomputation costs nothing but a re-read.

So an anchor set is its own blob, keyed by the observation digest plus the pack
digest, and a typebank is valid with none, some, or all of its observations
anchored.

**The `Fit` and kind distribution is a gap ledger for the grammar/compiler
seam**, in the same sense treebank already ledgers grammar gaps against its
oracle. 0.45% for C against git is a number; the same number for C# against
`dotnet/dotnet`, or for TypeScript against DefinitelyTyped, is worth having
before anyone trusts a cross-tool join in those languages.

### What anchoring does not fix

The measurement above compares facts clang *produced*. It is silent about facts
clang could not produce, and that is where the hard case lives.

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

A call inside an inactive branch is not a bad anchor. It is an **absent
observation** — the compiler never saw the text, so there is no fact to anchor.
No amount of node matching surfaces it, and the git measurement cannot detect
it either: 14 of those 40 files parsed with `ERROR` nodes present and their
calls still anchored cleanly, because every call clang reported came from the
configuration clang read.

That is a coverage problem, and it is handled where coverage problems belong:

- `Span` carries a `text_digest` — a hash of the unit as *that producer read
  it*. A producer that read rewritten, reduced or preprocessed text has a
  different digest, and a consumer joining across a digest boundary gets a
  diagnostic rather than a silent mismatch.
- `Environment::configuration` records the preprocessor symbols, `cfg` flags or
  target in effect. Two observations of one file under two configurations are
  **two observation sets**, joinable with each other and not with a parse that
  saw all branches.
- *Intended, unverified:* `Span` gains `expanded_from: Option<Box<Span>>`. clang
  exposes spelling and expansion locations separately
  (`clang_getSpellingLocation` / `clang_getExpansionLocation`), which is what
  would let a fact about generated text join back to the macro the author wrote.
  The git measurement suggests this matters less than expected — 97.8% of
  macro-expanded calls anchored exactly — but the remaining 2.2% is where it
  would earn its place.

cowbird's `cgraph.rs` already lists "conditional-compilation branches the
canonical parse did not see" among the things it knowingly misses. Semiotics
should make that visible in the data rather than in a module comment, and the
anchor is not the mechanism that does it.

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

**Why anchors are keyed separately.** An anchor blob's name is
`sha256(observation_digest ‖ pack_digest)`. treebank ships new packs and closes
grammar gaps continuously, so anchors are recomputed often; observations need a
working toolchain and a build, so they are recomputed rarely. Fusing them would
mean re-running a compiler to pick up a grammar fix. Separate keys also make the
`Fit` distribution diffable across pack versions, which is what turns it into a
ledger rather than a one-off measurement.

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
2. **`semiotics-c` and `semiotics-store`, consumed by cowbird's `cgraph.rs`,
   with anchoring alongside.**
   The first slice that matters. C is the one language where the observer is
   nearly free to write, and cowbird already has an `nm`-scored ≥95% / ≥90% bar to
   pass or fail against. If it clears, the design is load-bearing; if not, we
   learn that before anything is built on top.
3. **`entl-tree-sitter` → `semiotics-parse`.** The largest mechanical move, and
   the one that breaks cowbird and infact, so it goes after the seam has been
   exercised once. treebank narrows to automation in the same pass, keeping only
   what writes packs.
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
- **Anchoring, over the first 40 `.c` files of a git checkout** (libclang 20.1.2
  against tree-sitter-c via `tree_sitter` 0.26, the same tree-sitter major Entl
  pins): 8,431 calls, of which 8,392 (99.5%) land on an exact byte match to a
  `call_expression` node, 38 (0.45%) on an exact match to a different node kind,
  1 contained-but-not-exact, and **none** straddling a boundary or missing a node.
  1,716 calls (20.4%) are macro expansions and 1,678 of those anchor exactly. All
  38 kind mismatches are macro expansions, and the archetype is `errno` reading as
  an `identifier` to the grammar and as a call to `__errno_location()` to clang.
  14 of the 40 files parsed with `ERROR` nodes present without affecting the fit.
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
2. **Is entl being non-standalone acceptable?** Entl depends on `semiotics` for
   `LanguageId` and detection, and on nothing else. The alternative — entl keeps
   recognition, semiotics takes meaning — buys standalone-ness for two
   registrations per language across two repositories, and a drift failure that
   is silent. Argued in *Why entl depends on semiotics*; the most reversible
   decision in this document, and the one most worth a second opinion.
3. **Does `LanguageProfile` split as cleanly as the line count suggests?** The
   2,225/1,142 measurement says the modules do not reference each other, but
   `model/id.rs` defines language and package identifiers together and
   `LanguageDetection` is stored on `FileEntry`. That boundary is the one place the
   split could turn out to be more than a move.
4. **Do dialect rewrite tables become pack data?** They live in
   `semiotics-parse` as code today. Shipping them in the pack would record a gap
   and its workaround together and let treebank emit both, at the cost of a
   richer pack format. Not decided.
5. **What is the `Fit` distribution in a language with a real preprocessor
   problem?** C against git is 99.5% exact, but C is the language where the two
   tools agree most. The number that would change the design is C# against
   `dotnet/dotnet`, where treebank has already measured 4,617 files whose grammar
   failure is caused only by conditional compilation. Cheap to run once a C#
   observer exists, and it should run before any consumer trusts a cross-tool join
   in C#.
6. **Should an observation declare the node kind it expects?** The measurement
   says kind mismatch, not structural misfit, is where the disagreements live —
   which suggests a `CallEdge` should say it expects a `call_expression` so the
   anchor can flag `errno`-shaped cases automatically. That means the schema
   naming grammar node kinds, which couples it to a specific pack's vocabulary.
   Undecided, and the coupling is the reason.
7. **Is tier 1 worth having for anything but C?** Every tree-sitter observer is
   tier 1, but tree-sitter observations are not compiler observations. If C is the
   only compiler in tier 1, the tier distinction may be one language wearing a
   general name.
