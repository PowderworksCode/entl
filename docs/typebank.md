# Typebank

*A design sketch. Nothing here is implemented. Where a claim was tested, the
probe and its numbers are given; where it was not, it is marked unverified.*

Typebank exports what a language's own tooling knows about a program — resolved
calls, types, definitions, references — anchors every fact to a byte range, and
records honestly what it could not answer. It is the destination for the
parsing and observation code Entl is going to shed.

This document follows Entl's convention: *intended* marks something proposed
rather than existing.

## The premise, corrected

The idea that motivates typebank is that treebank already runs compiler front
ends and keeps one bit, so the discarded remainder is nearly free.

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

**Semantics is cheap only if the unit changes from file to project.** The whole-
project column is 2.6× to 19× the validity bit, which is affordable. The per-file
column is 34× to 687×, and worse than slow: C# per-file resolves only 1,000 of
1,200 invocations, silently losing every cross-file call. treebank's unit is the
file, because it sweeps corpora that do not build — 860,590 `.cs` files from
monorepo checkouts, Debian source tarballs, npm tarballs. Typebank's unit is the
project, because that is the only unit where a checker is both correct and
affordable. **They cannot share an invocation.**

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

So typebank does not inherit treebank's compiler runs. **It inherits treebank's
adjudication discipline**: the three-valued verdict, categories rather than
message text, `other` as a named tripwire, "never invent invalidity", the
negative corpus, and a ledger that states what was not adjudicated. That
discipline is a more developed `Fidelity` than Entl's `Fidelity`, which does not
exist in code.

## Disposition

For each Entl crate, where it goes.

| crate | disposition |
|---|---|
| `entl-codebase` | **stays in Entl**, splits out `entl-language` |
| `entl-github` | **stays in Entl** |
| `entl-semantics` | **moves to typebank**, becomes `typebank-schema` |
| `entl-rust-mir` | **moves to typebank**, intact |
| `entl-ts-observe` | **moves to typebank**, intact |
| `entl-zig-observe` | **moves to typebank**, intact |
| `entl-zig-air` | **splits**: reader → typebank observer, `store.rs` → `typebank-store` |
| `entl-tree-sitter` | **splits three ways**: manifest → treebank, runtime → typebank, `repository.rs` deleted |

The dependency graph makes most of this mechanical. Today:

```
entl-codebase   ←  entl-github
                ←  entl-tree-sitter  ←  entl-zig-observe
entl-semantics  ←  entl-rust-mir
                ←  entl-ts-observe
entl-zig-air    (arrow, parquet — no entl dependency at all)
```

`entl-semantics` has **zero dependencies**. Everything hanging off it moves for
free. The whole split turns on one crate.

### The obvious ones

**`entl-codebase` stays.** Inventory is what Entl is keeping: files, languages,
manifests, packages, workspaces, projects. Nothing in it observes anything.

**`entl-github` stays.** Forge facts are acquisition. It carries no span and runs
no compiler. Its known conflation of repository vocabulary and GitHub adapter is
a real problem and is unaffected by this split.

**`entl-semantics` moves and becomes typebank's root.** Zero dependencies means
the move is a `git mv`. It is already the right shape — see *A common format is
an envelope* — and per the brief's default it crosses largely intact, with three
amendments named later: byte spans, `Fidelity` plus `Environment`, and stable
entity ids.

**`entl-rust-mir`, `entl-ts-observe`, `entl-zig-observe` move intact.** They are
observers. `entl-zig-observe` is worth noting because it observes through
tree-sitter rather than a compiler: it extracts Zig container fields with the
type "exactly as the author wrote it", precisely because the grammar mis-groups
`*jsc.VirtualMachine` as a pointer to `jsc`. Observer does not mean compiler, and
this crate is the proof.

### `entl-zig-air` splits

`air.rs` (509 lines reading `zig build-obj --verbose-air` output) is an observer
and moves as one. It currently emits its own Parquet schema, bypassing
`entl-semantics` entirely — the Entl design doc already lists this as a defect
under Direction item 2, and it should arrive speaking the envelope.

`store.rs` (286 lines of Arrow/Parquet writing) is **not a Zig fact**. It is a
storage backend that happens to have been written inside a Zig crate because Zig
was the first observer with millions of rows. It becomes `typebank-store`, owned
by no language, and is the reason typebank does not need to invent a columnar
sink from scratch.

### `entl-tree-sitter` splits three ways

This is the only genuinely contested crate, and it is contested because three
different things share one name.

**`manifest.rs` → treebank**, as `treebank-pack`. It defines the `parser.toml`
schema — `schema`, `id`, `language`, `version`, `source`, `revision`, `license`,
`abi`, `sha256`, `[files]`, `[tokenization]` — and verifies the digest. treebank
*writes* that file; this code only *reads* it. A format belongs with its producer,
and treebank is the only thing that can change it. As a separate crate it carries
no tree-sitter runtime dependency and is publishable alongside the packs.

**`catalog.rs` + `runtime.rs` + `dialect.rs` → typebank**, as `typebank-parse`.
This is parse execution: pack loading, query compilation, the parser runtime, and
the 1,533-line per-language dialect rewrite table. It belongs with observation
because `ParsedFile::rewrites_narrowed` — the only fidelity signal that exists in
code anywhere in this fleet — is produced here, and fidelity belongs beside the
observations it qualifies.

*Intended:* the dialect rewrite tables should eventually ship **in the pack** as
data rather than living in a crate, and treebank should emit them, because
treebank is what discovers a grammar gap and already ledgers it. Today the same
knowledge is duplicated: treebank knows `async` is a C# contextual-keyword gap
because it patched the grammar for it, and `dialect.rs` separately carries
rewrite rules for gaps it could not patch. Naming the seam now is cheap.

**`repository.rs` → deleted.** These 148 lines are `parse_repository(root,
catalog)`, a convenience driver that calls `entl_codebase::walk()` and loops. They
are also the *entire* reason the observation half depends on the inventory half.
They are replaced by the seam below.

## The seams

### What the coupling actually is

Entl's design doc argues against this split directly:

> "They belong in one library because acquisition is the shared problem.
> Inventory decides which files exist and what language each is; observation
> needs that answer before it can choose a grammar or a compiler."

Then, four lines later:

> "Nothing in `entl-semantics` depends on `entl-codebase`."

I checked every edge. `entl-tree-sitter` is the only crate on the observation
side that touches `entl-codebase`, and it touches it in exactly three places:

1. `repository.rs` — `walk()`, to get a file list.
2. `catalog.rs` — `language_profile(&manifest.language)`, to reject a pack naming
   an unknown language.
3. `catalog.rs` — `role.expects_parser_pack()`, to decide whether a missing pack
   is a diagnostic or silence.
4. `error.rs` — the `Error::Codebase` variant that follows from (1).

The file list is a **value**, not a dependency. So is the project root that
`entl-ts-observe --project` needs. The doc's argument describes a real ordering
between layers and mistakes it for a reason to share a crate.

The coupling that genuinely *is* a code dependency is **language identity**, and
the doc never mentions it. That is what survives scrutiny, and it is why the
answer is not a clean amputation.

### `entl-language`

*Intended:* split the language identity registry out of `entl-codebase` — the
`LanguageId`, `LanguageProfile`, ecosystem role, and the registration mechanism —
with no walker, no manifest parsing, no `ignore`, no `globset`, no `toml`.

```
entl-language          LanguageId, LanguageProfile, role, registry
   ↑                   small, stable, few dependencies
   ├── entl-codebase   (Entl)
   └── typebank-parse  (typebank)
```

Both sides depend on it; neither depends on the other. Without this, typebank
either pulls in the whole inventory crate for a name registry, or forks the
language list and the two drift.

### Entl → typebank

The seam is a value type, not a trait:

```rust
// typebank-schema (intended)
pub struct ObservationUnit {
    pub path: PathBuf,
    pub language: entl_language::LanguageId,
    pub read: Box<dyn Fn() -> io::Result<Arc<[u8]>> + Send + Sync>,
    /// The build the observer should configure itself from, when there is one.
    /// `None` means per-file observation is the only thing available.
    pub project_root: Option<PathBuf>,
    pub toolchain: Option<ToolchainId>,
}

pub fn observe(units: impl IntoIterator<Item = ObservationUnit>) -> ObservationSet;
```

That struct is exactly what `parse_repository`'s loop already extracts from a
`CodebaseTree`, plus the project root that `inspect` resolves. Making it a value
means typebank can be driven by Entl's inventory, by a git commit tree, by
treebank's corpus manifest, or by a directory listing — and it retires the
`entl-codebase` dependency from the parse path.

`project_root: Option<_>` is the axis of the whole design in one field. Entl's
`inspect` is the only thing that turns a directory into a set of projects, so
Entl is what makes the expensive-but-correct column of the cost table reachable.
treebank's corpus cannot supply it — a Debian source tarball has no configured
build — which is exactly why treebank stays per-file.

A ~30-line adapter crate `typebank-entl` provides
`units_from(&CodebaseInventory) -> Vec<ObservationUnit>`, so cowbird and infact
do not break. It lives in typebank's workspace. **Entl gains no dependency on
typebank.** The direction is one-way and stays that way.

### treebank ↔ typebank

*Intended, and this is the "combination of the oracle function from treebank" the
original idea asked for:* the **oracle interface moves to typebank; the corpus-
sweep policy stays in treebank.**

typebank owns "run this language's front end over this file with this
environment, and return both the adjudication and whatever was observed".
treebank calls it and keeps one bit for its gap ledger. That inverts today's
ownership, and it is the right direction: typebank produces facts, treebank
consumes one of them.

The cost is real and bounded: treebank's sweep must stay fast over 850,000 files,
so the observer needs a **verdict-only mode** that builds the translation unit and
skips the AST walk. My C probe shows the TU is constructed either way; the walk is
the only extra work, so the mode is nearly free.

`Lang::rank` / `resolve` / `classify` / `grammar_dirs` / `route` stay in treebank
untouched — that is corpus acquisition for grammar work, not codebase inventory.

### Where the shared shapes live

**They turn out not to be shared.** After the shed, Entl produces no span-anchored
fact: `ParseDiagnostic` is `(path, message)`, `LanguageDetection` is evidence
without offsets, `entl-github` records workflow locations rather than byte ranges.

So `Span`, `Provenance`, `Fidelity` and `Environment` all live in
**`typebank-schema`**, and Entl needs none of them. If Entl later wants provenance
on inventory facts it will want a *different* provenance — source tree identity,
not toolchain identity — and duplicating a four-field struct is cheaper than a
shared crate for it.

treebank keeps its own span for the gap ledger. Its ledger is a corpus artifact
rather than a fact stream, and forcing it through typebank's schema buys nothing.

The one shape that really is on both sides is `LanguageId`, which is why
`entl-language` exists and why it is the only new crate Entl gains.

## A common format is an envelope

**Envelope.** Not an IR. Entl's design already settles this and it settles it
correctly; the job here is to say why the evidence supports it, and to correct one
thing.

### Against an IR

Each producer holds something the others cannot express, and I have four
instances in hand rather than an argument from principle:

- **clang** hands over a USR and a resolved type at a byte offset. Verified.
- **tsc** hands over a structural type with no constructor name at all —
  `entl-ts-observe` is already forced to emit `head: "(anonymous)"` for it.
- **rustc MIR** hands over monomorphized instances, which a source-level schema
  has no slot for. `Dispatch::Unmonomorphized` exists as a warning rather than a
  fact precisely because the mismatch could not be normalised away.
- **`entl-zig-observe`** carries the type as *raw source text*, because the
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
residue it describes as "gaps in the index, and C FFI types". Those are typebank
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
  saying which were attempted. This is *not* an IR: it normalises the **questions**,
  not the answers. `CallEdge::to` is a `Vec` for exactly this reason, and the
  comment on it is the best statement of the principle in the codebase.
- **The open tail** is the change. Today it is `Vec<Gap>` and a `Gap` is a string.
  *Intended:* a producer-keyed typed payload, so `ContainerField`'s dotted
  container path, clang's include environment, and MIR's terminator kinds stop
  being unrepresentable and stop being flattened into prose.

The name `SemanticObservations` is right and should not change for the sake of a
new repository.

## Scope per language

Which unit each language's tooling actually works on. This is the distinction the
brief expected to decide the design, and it does.

| language | mechanism | works per file? | works per project | native stable entity id |
|---|---|---|---|---|
| **C** | libclang parse-only TU | **yes**, degrading with the include environment | better, with `compile_commands.json` | USR (`c:@F@mylib_len`) — verified |
| **Java** | `JavacTask.analyze()` | technically; 34× and errors on every cross-file reference | yes, 2.6× over parse | binary name via `Elements` — unverified |
| **C#** | `CSharpCompilation` + `GetSemanticModel` | **no** — 79×, and silently loses 200 of 1200 calls | yes, 19× over parse | `GetDocumentationCommentId()` — verified |
| **TypeScript** | `ts.createProgram` + checker | **no** — 687×, reloads `lib.d.ts` per file | yes, 16× over parse, needs `tsconfig.json` | **none** — see below |
| **Rust** | `entl-rust-mir` as a drop-in `RUSTC` | **no** — there is no per-file mode at all | requires a working `cargo build` | rustc def path |
| **Zig** | `--verbose-air` from a forked toolchain | unverified | requires a build | unverified |

Reading:

- **C is the only language where per-file observation is a first-class mode**, and
  even there the verdict is relative to the include environment, which
  `ORACLE.md` already says out loud.
- **Rust has no cheap tier.** treebank's oracle is `syn`, a parser crate with no
  resolver, so there is nothing to extend — Rust semantics means running a
  different program under a real build. The 16×-to-19× framing does not apply;
  the cost is "a build succeeds or you get nothing".
- **Java's 2.6× is the cheapest semantic tier in the fleet.** Attribution over an
  already-parsed unit is close to free. If any language nearly vindicates the
  original premise, it is Java, and only when the unit is the whole compilation.
- **TypeScript mints no stable entity id.** `entl-ts-observe` falls back to
  `path:line:column` for local declarations, which changes when a blank line is
  inserted above. That cannot be a cache key or a cross-version join key. C and C#
  both hand over stable ids natively (verified). *Intended:* an `EntityId` must be
  stable under whitespace change — use the language's native id where one exists,
  and a structural path (`module::container::name#overload-index`) where none does.

Practical consequence: **typebank has two tiers, and an observation must say
which it came from.**

- **Tier 1, per file, no build.** C via libclang; every tree-sitter-based observer.
  Cheap, sweepable over corpora that do not build, and the only tier treebank can
  use.
- **Tier 2, per project, build required.** Rust, TypeScript, C#, Java, Zig.
  2.6×–19× the validity bit, correct, and unavailable for most of any corpus.

## Fidelity and loss

Entl's *intended* `Fidelity` — exact / degraded / narrowed / unresolved — is the
right instinct and is not sufficient. The C probe shows why: a call whose header
was missing came back with a **correct USR and the wrong type**. Nothing in the
observation distinguishes it from the good case. There is no fidelity level an
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
    /// includes, imports, classpath entries, `node_modules`, sysroot.
    pub inputs_resolved: u32,
    pub inputs_missing: Vec<String>,
    /// The producer's own error counts under its own category names —
    /// clang's `Parse Issue` / `Semantic Issue` / `Lexical or Preprocessor
    /// Issue` / `User-Defined Issue`, and their equivalents. Counts, never
    /// message text: `ORACLE.md` shows classifying prose does not work.
    pub diagnostics: BTreeMap<String, u32>,
    /// The configuration observed: preprocessor symbols, cfg flags, target
    /// triple, `tsconfig` path. Two configurations of one file are two
    /// observation sets, not conflicting facts about one.
    pub configuration: BTreeMap<String, String>,
}
```

And one invariant, which is checkable rather than aspirational:

> **An observation from a unit whose `Environment` reports missing inputs may not
> be presented as `Exact`.**

That single rule catches the clang trap, which no per-observation field can. It is
also directly inherited from `ORACLE.md`'s categorical rule — "any semantic or
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

1. **`inputs_digest`** — content hash of the bytes observed. Without it, staleness
   is a filesystem timestamp question.
2. **`pack_digest`** — what `queries_sha256` already does for parser packs, so two
   observer packs wrapping the same toolchain stay distinguishable.
3. **`resolution_digest`** — hash of the resolved dependency set (`Cargo.lock`,
   `package-lock.json`, `compile_commands.json`, the classpath). **A `cargo update`
   changes MIR without changing one byte of source in the unit.** This is the field
   whose absence would bite hardest and it is the least obvious.

With those, staleness is a comparison rather than a heuristic:

| mismatch | meaning |
|---|---|
| `inputs_digest` | the source changed — re-observe |
| `resolution_digest` | dependencies moved — re-observe |
| `toolchain` | a different compiler answered — re-observe, and the old facts may still be worth diffing |
| `provider_version` | the extractor changed — re-observe |
| `schema` | the consumer must decide; a bump is a migration |

The MIR question specifically: a consumer knows a MIR observation is stale because
the `toolchain` string does not match the toolchain now installed. It cannot know
whether MIR's *shape* changed underneath — that is what `provider_version` plus a
pinned toolchain is for, and it is why `entl-rust-mir` carries its own
`rust-toolchain.toml` and always will.

## Joining with tree-sitter

Spans are the join key, and today they do not join. Two problems, one verified
defect and one genuine disagreement.

### `Span` must carry bytes

`entl_semantics::Span` is `path` + line/column. The design doc says "bytes 100 to
200"; the code says line 4 column 7. Worse, `entl-ts-observe` derives its columns
from TypeScript's UTF-16 offsets. Verified, on
`const s = "café 🎉"; export const n = s.length;`:

| producer | reports |
|---|---|
| `entl-ts-observe` | column **39** (UTF-16 code units) |
| tree-sitter, clang | byte **41** / column **42** |

Three units of skew from two non-ASCII characters, silently, on the join key for
the whole system. *Intended:* `Span` carries `start_byte` / `end_byte` as the
identity, with line/column alongside for display only. Every producer in the
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
  A producer that read rewritten, reduced, or preprocessed text has a different
  digest, and a consumer joining across a digest boundary gets a diagnostic, not a
  silent mismatch.
- `Environment::configuration` records the preprocessor symbols, `cfg` flags, or
  target in effect. Two observations of one file under two configurations are
  **two observation sets**, joinable with each other and not with a parse that saw
  all branches.
- For macro and generated code, *intended, unverified:* `Span` gains
  `expanded_from: Option<Box<Span>>`. clang exposes spelling location and
  expansion location separately (`clang_getSpellingLocation` /
  `clang_getExpansionLocation`), which is what would let a fact about generated
  text join back to the macro the author wrote. I did not probe this.

This costs consumers something honest: a query spanning a preprocessor boundary
returns a gap instead of an answer. That is the correct outcome. cowbird's
`cgraph.rs` already lists "conditional-compilation branches the canonical parse
did not see" among the things it knowingly misses; typebank should make that
visible in the data rather than in a module comment.

## Consumers

A format with no named consumer is speculation. Both existing consumers have
queries they cannot answer today, and each has already built a worse version of
typebank inside itself.

### cowbird `src/analysis/cgraph.rs`

Today it reconstructs the C dependency graph — "which file defines the symbols
this file references" — from syntax: a definition index of non-`static` functions
and file-scope variables, plus identifiers a file uses but does not bind. It is
scored against `nm` over actually-built objects, with an acceptance bar of ≥95%
precision and ≥90% recall, and it names three things it knowingly misses.

**The query it could not answer before:** *for a symbol introduced by macro
expansion, which translation unit defines it?* `cgraph.rs` lists
macro-generated definitions (`define_commit_slab`) as a known miss — the dialect
table blanks their sites so files parse, and the generated names resolve nowhere
and land in `unresolved`. libclang resolves them, because it observes after
expansion. Verified that the mechanism exists (USR + resolved cursor from the
parse-only TU); not verified against cowbird's corpus.

This is the strongest case in the fleet: the tier-1, per-file, no-build C observer
is exactly what cgraph needs, and it is the one language where the cost is
genuinely near zero.

### cowbird `src/analysis/typemap.rs`

Measured over Bun's 61,538 signature type positions, the rewriter resolves ~70%.
The residue includes 6.9% `unknown-name` — "gaps in the index, and C FFI types".

**The query:** *what does this name resolve to, when the index does not have it?*
A Zig observer answers the index gaps directly. The C FFI half is answered by the
C observer, on the other side of a language boundary the current design cannot
cross at all. Note what it does **not** answer: 14.6% `pointer-needs-class` is
ownership, which `typemap.rs` correctly says "is not a property of the type at
all". Typebank must not pretend to answer it.

### cowbird `src/analysis/lsp.rs`

A hand-rolled LSP client driving zls, with its own `Outcome` enum —
`Resolved` / `Timeout` / `Empty` / `BadUri` / `NoResult` — so a failure can be
attributed instead of lumped under "zls is slow".

That is an observer pack with a fidelity enum, written in a consumer because there
was nowhere else to put it. It becomes `typebank-zls`, and its `Outcome` maps onto
`Fidelity` almost term for term. This is the clearest evidence that typebank's
shape is right: someone already built it, in the wrong place, and arrived at the
same abstractions.

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

Becomes a consumer of the verdict-only mode, as described at the treebank seam. It
gets the same adjudication it has today with one fewer copy of the oracle to
maintain.

## Storage and shape on disk

A typebank is **content-addressed blobs plus a small index**. Not a database.

```
typebank/
  observations/
    sha256-<digest>.json          # the envelope: provenance, coverage,
                                  # environment, and the closed-core facts
    sha256-<digest>.parquet       # optional columnar sidecar, same digest,
                                  # for high-cardinality payloads
  index.jsonl                     # (path, provider, digest, spans-range) per line
```

**The digest is the cache key and the staleness check in one:**

```
digest = sha256(inputs_digest ‖ resolution_digest ‖ provider ‖ provider_version
                ‖ toolchain ‖ pack_digest ‖ canonical(configuration) ‖ schema)
```

Every input to that hash is a field the previous two sections argued must exist
anyway. A consumer that can compute the key knows without reading anything whether
its cached answer is still good, which is the property `entl-tree-sitter`'s
provenance work was a precondition for.

`SemanticObservations::canonicalize()` already sorts and dedups every collection
so that the same source and toolchain produce byte-identical output. That is what
makes content addressing work at all, and it exists today.

**Why not a database as the primary artifact.** It is not diffable, not content-
addressed, and not reviewable. More concretely, infact already runs DBSP for
incremental relations — typebank should feed that, not compete with it. A
consumer wanting SQL builds a projection; the Entl design doc's Direction item 8
says the same thing about serialization, and for the same reason.

**Why both JSON and Parquet.** `entl-zig-air` already writes Parquet because AIR
instruction counts run to the millions, and that was the right call. The envelope
is small and belongs in reviewable JSON; a payload with millions of rows belongs
in a columnar file. One digest names both, so the pair is atomic. This is what
`typebank-store` is for, and it is why `entl-zig-air`'s `store.rs` should be
lifted rather than deleted.

**Why it is cheap to query.** Facts sorted by `(path, start_byte)` make a span
range query a binary search, and `canonicalize()` is already half of that sort.
The index carries each blob's path set and byte range, so "what does anything know
about `src/lib.rs:150`" reads one index line and one blob rather than scanning.

**Why it is safe to cache.** The digest covers the toolchain and the resolved
dependency set, not just the source. That is the difference between a cache that
is safe and one that is merely fast.

## Where I disagree with Entl's design

Stated directly, as the brief asks.

1. **`Span` is line/column and must be bytes.** Verified skew between
   `entl-ts-observe` and every byte-native producer, on any line containing
   non-ASCII. The design doc's prose is right and the code does not implement it.
2. **`Fidelity` alone is insufficient.** It cannot express "the answer is present
   and wrong", which is the failure the C probe produced and the one the design
   doc itself calls the dangerous kind. `Environment` plus the not-`Exact`
   invariant is the fix.
3. **"Two domains, one library" defends a real relationship in the wrong form.**
   The coupling is not inventory — that is a value. It is language identity, which
   the doc does not mention. `entl-language` is the smallest thing that honours
   the real constraint.
4. **`EntityId` has no stability requirement**, and `entl-ts-observe` mints
   `path:line:column` for local declarations, which changes when a blank line is
   inserted above it. C and C# hand over stable ids natively (verified). The schema
   should require stability under whitespace change.
5. **`Coverage` merging destroys information.** ANDing across units means one
   unbuildable package erases a true claim about the rest. Keep it per unit.

Where the design is right and should not be relitigated: the rejection of a common
IR; spans as the join key; provenance on every fact; the pack mechanism and its
runtime discovery; the wasm boundary; and treating an absent compiler as a
diagnostic rather than a silent empty result. Those hold up, and typebank inherits
them unchanged.

## What was verified

**Run on this machine.**

- TypeScript 5.9.3, 200 interlinked files: 54 ms parse-only / 866 ms whole-project
  checker / 36,837 ms per-file checker.
- C# Roslyn 4.8 via SDK 8.0.129, 200 files: 30 ms / 557 ms / 2,364 ms, with
  per-file resolving 1,000 of 1,200 invocations and reporting 200 errors.
- Java javac 21.0.11, 200 files: 129 ms / 339 ms / 4,364 ms, with parse-only
  resolving **zero** invocations.
- libclang 20.1.2, parse-only with `KeepGoing`: resolved callee, USR, type and byte
  offset present; with the header absent the same call reports the same USR and
  type `int` instead of `mylib_size_t`.
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

**Read, not run.** Every Entl crate's dependency edges; treebank's five
`validate()` implementations and the `add-c-grammar` libclang oracle; `ORACLE.md`'s
gcc-versus-libclang table and its four-category rule; `LOCAL-PATCHES.md`'s 4,617
inherent / 2,531 actionable split; cowbird's and infact's dependency sets and the
module comments quoted above.

**Not verified.** rustc MIR extraction cost (needs the pinned
`nightly-2026-07-18` toolchain). Zig AIR (no `zig` on this machine). javac's
`Elements` binary names as a stable entity id. clang's spelling-versus-expansion
locations for the macro join. Whether the C observer's edges actually clear
cowbird's ≥95%/≥90% bar on a real corpus — that is the single most valuable
unverified claim here, and it is the one to test next.

## Open questions

1. **Does the C observer clear cowbird's `nm` bar?** Everything in the consumer
   section rests on it. Testable today against the corpus cowbird already scores.
2. **Does treebank accept the inverted oracle dependency?** It is the right shape
   and it makes treebank's sweep depend on a crate outside itself. If the answer is
   no, treebank keeps `tools/c-oracle` and typebank grows a near-duplicate — about
   200 lines of C, which is a real but survivable cost.
3. **Where do dialect rewrite tables ultimately live?** Proposed above as pack data
   emitted by treebank, kept as typebank code for now. The duplication with
   treebank's patch series is small today and grows with every language.
4. **Does `entl-language` split cleanly?** Asserted from the three call sites in
   `catalog.rs`; not attempted. If `LanguageProfile`'s facets and conventions turn
   out to be entangled with manifest parsing, the split is more than a move.
5. **Is `Tier 1` worth having for anything but C?** Every tree-sitter observer is
   tier 1, but tree-sitter observations are not compiler observations. If C is the
   only compiler in tier 1, the tier distinction may be one language wearing a
   general name.
