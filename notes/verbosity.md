# entl — language verbosity: what was measured, and what it turned out to mean

Written after building `tools/verbosity` and the three corpus readers. The
generated reports under `docs/verbosity-*.md` say what the numbers *are*; this
says what they are *worth*, which is a different and less comfortable question.

## The ask

Compute a matrix of relative verbosity between the profiled languages and carry
it in the language profiles. Normalize on one language if the ratios permit it —
noting up front that they might not, since C-to-Java need not equal
(C-to-Lisp)(Lisp-to-Java).

They don't permit it cleanly, and the ways in which they don't turned out to be
more interesting than the matrix.

## Licensing came first, and it shaped everything

Rosetta Code is GFDL 1.2. Its own copyright page says the material "is not
compatible with most software licenses, including OSI-approved licenses such as
the GPL", which includes Entl's MIT.

The line drawn: **numbers cross, text does not.** Counts, ratios, and a fitted
index are measurements about a work, not the work. No corpus source is copied
into the repository, quoted in a fixture, or embedded in a doc example. The tool
reads a checkout the operator downloads.

That discipline was adopted under duress and then paid for itself. Exercism
(MIT) and mal (MPL-2.0) were added later with no licensing conversation at all,
because the boundary was already in the right place. If the first corpus had
been permissive, the tool would probably vendor snippets today and adding a
GFDL corpus would have meant unpicking it.

## Three corpora, and why more than one

| corpus | units | control | license |
| --- | --- | --- | --- |
| Rosetta Code | ~1,750 tasks | none — anyone contributes, entries answer different questions | GFDL 1.2 |
| Exercism | ~166 exercises | one reference solution per track against a shared spec and test suite | MIT |
| mal | 1 interpreter | one 11-step guide, one test suite, all 16 languages | MPL-2.0 |

Exercism drives the checked-in table. It is the best-controlled: `deviation`
(how far one index per language sits from the directly measured pairs) has a
median of **5%** against Rosetta's **12%**, and its thinnest language pair
shares **70** units against Rosetta's **27**.

Rosetta and mal are published as cross-checks rather than discarded, because the
disagreement between them is the most useful thing here.

## Finding 1 — verbosity is a property of the corpus at least as much as of the language

Bytes relative to C:

```
language     exercism    mal   rosetta
ruby             0.43   0.25      0.40
python           0.41   0.56      0.62
rust             0.59   0.59      0.92
java             0.82   0.79      1.05
c                1.00   1.00      1.00
zig              0.66   1.24      1.39
```

Zig is 0.66, 1.24, or 1.39 depending only on which corpus you ask. Python nearly
doubles. The **spread** — most verbose over least — is 2.5× on Exercism, 3.5× on
Rosetta, 4.9× on mal.

The direction is not random. Exercism's units are function-shaped, so they
mostly measure the *absence of ceremony*: whether a language can express a task
without declarations, imports, or a class to hold `main`. mal's unit is a
working interpreter of a few thousand lines, which has structure every language
must pay for. Small-task corpora compress the range.

An index without its corpus named is not a fact. `VERBOSITY_CORPUS` and
`VERBOSITY_CORPUS_REVISION` are in the generated table for this reason.

## Finding 2 — the ordering is stable even when the magnitudes are not

Spearman rank correlation between corpora: mal↔Rosetta **0.87**, mal↔Exercism
**0.80**, Exercism↔Rosetta **0.79**.

So "Ruby is terser than Java" is robust across every instrument tried. "Ruby is
0.43× C" is a fact about Exercism. Consumers should lean on the ordering and on
`verbosity_ratio` for specific pairs, and treat the index as a ranking device.

## Finding 3 — non-transitivity is real, and it is task selection

Each pair is averaged over a different slice of the corpus, so the ratios do not
compose. Worst triangle on Rosetta: `php`→`typescript`→`zig` composes to 0.613
against 0.368 measured directly, a 67% gap on a 27-task leg.

Restricting to the balanced panel — units every language implements — makes the
ratios transitive by construction. The index moves toward the baseline for
*every* language when you do that, because the tasks everyone implements are the
short ones where fixed ceremony is a larger share. That gap is composition bias,
and it is reported in each corpus's document.

## Finding 4 — writing from a spec and translating existing code are different quantities

This one cost the most to learn, and it was learned by being wrong.

A downstream consumer estimates the cost of porting a Zig codebase to Rust and
assumes Rust output is **1.37×** the Zig input in tokens. Measured on Rosetta,
independently written Rust runs about **0.71×** Zig — the opposite direction. On
that basis the constant was judged roughly 1.9× too high.

It was not. Bun's Zig-to-Rust rewrite had already shipped, and at the merge
commit both trees sit side by side, 1,231 files at identical relative paths.
Measured there, in the same token encoding:

```
rust/zig = 1.374   (geometric mean over 1,231 paired files, CI [1.32, 1.43])
```

Rosetta was measuring idiomatic reimplementation from a specification. The
consumer was estimating faithful translation of existing structure. Those differ
by about 1.9× for this language pair, and no amount of care with the corpus
would have closed the gap, because it was the wrong instrument.

**When the thing being estimated has actually happened somewhere public,
measure that before building a proxy.**

Entl's table therefore says, in the generated header and in the README, that it
is not a porting factor.

## Finding 5 — port expansion is comments, not code

Strip comments from both sides of the same 1,175 Bun file pairs:

```
with comments (tokens)       1.376   CI [1.321, 1.433]
comments stripped (tokens)   1.012   CI [0.968, 1.058]
comments stripped (bytes)    0.982
```

The ported Rust is the same quantity of *code* as the Zig it replaced. Comment
share went 20.7% → 37.4%; comment tokens multiplied by 2.5.

Bun's `docs/PORTING.md` never asks for this. Two mentions of "comment" in 575
lines, one of which is a fixed eight-line `PORT STATUS` trailer that accounts for
2.5% of the Rust's comment mass. Doc comments — never requested — more than
doubled. Human authorship does not explain it either: 111,091 additional comment
lines in 11 days.

## Finding 6 — human porters preserve comment density; the model overwrites it

Eleven ports, each paired per-file by git's own rename detection across an
extension change (which requires content similarity, so a detected rename is a
genuine translation of that file).

| port | pair | by | n | cmt src | cmt tgt | shift | code |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| bun | zig→rust | **LLM** | 1175 | 20.7% | **37.4%** | **+16.7** | 0.98 |
| prompt-codec | python→rust | **LLM** | 7 | 10.0% | **40.0%** | **+30.0** | 2.41 |
| typescript-go | ts→go | human | 10 | 26.0% | 23.0% | −3.0 | 1.07 |
| okhttp | java→kotlin | human | 96 | 48.2% | 47.5% | −0.7 | 0.98 |
| okio | java→kotlin | human | 16 | 50.7% | 50.3% | −0.3 | 0.96 |
| leakcanary | java→kotlin | human | 18 | 29.8% | 29.5% | −0.4 | 1.20 |
| jest | js→ts | human | 138 | 24.2% | 19.9% | −4.2 | 1.48 |
| puppeteer | js→ts | human | 60 | 24.5% | 27.2% | +2.7 | 1.13 |
| storybook | js→ts | human | 38 | 15.2% | 15.8% | +0.6 | 1.11 |
| redux | js→ts | human | 19 | 23.5% | 25.8% | +2.4 | 1.25 |
| date-fns | js→ts | human | 12 | 13.0% | 12.7% | −0.3 | 1.03 |

Human source comment shares span **13%–51%**, and every human target lands
within **4.2 points** of its source. The two LLM ports start 10 points apart and
both land at **37–40%**.

Humans track the source. The model substitutes its own default. `prompt-codec`
is the sharper test: its source was unusually sparse at 10%, it had the furthest
to travel, and it travelled furthest to the same destination.

Code-only ratios cluster by language pair, not around any universal constant:
js→ts geometric mean **1.19** (type annotations, as expected), java→kotlin
**1.04**, zig→rust **0.98**, ts→go **1.07**. A pair-level factor is good to
maybe ±20%; there is no porting constant.

## What this means for the API

- `LanguageProfile::verbosity` is a ranking device carrying a corpus reference.
  Prefer `verbosity_ratio(a, b)` when comparing two specific languages: it is the
  ratio as measured, not a quotient of two fitted numbers.
- `deviation` exists so a consumer can see how much the single index had to
  distort to reconcile the pairs. It is not decoration.
- The table is not a porting predictor and says so in its own header.
- Adding a language profile with comment syntax and a corpus mapping extends
  every corpus at once. The corpus covers far more languages than Entl profiles.

## What would firm this up

1. **Several mid-sized programs across the same languages.** mal is one program,
   so it is one genre — an interpreter leans on tagged unions, recursion, and
   manual memory for a garbage-collected target. Its disagreement with Exercism
   cannot be split into size effect and genre effect with n=1.
2. **More LLM ports.** The comment convergence rests on two, both Claude. The
   nine-port human baseline is solid; the other side is not yet.
3. **A pooled fit across corpora** with a per-corpus effect on the log ratios,
   so a corpus covering only some languages could still contribute, and the size
   effect became a term in the model rather than a caveat in prose.

## Corpora evaluated and rejected

- **Spec-implementation families** (Thrift, Avro, protobuf, Selenium, CommonMark,
  TOML). Scope is not controlled. TOML parsers — the tightest spec with wide
  coverage — span **29×**, from `tomli` at 51 KB to `Tomlyn` at 1,477 KB, because
  one is a bare parser and another carries serde integration and a formatter.
  Thrift's `lib/` runs TypeScript 29 KB against C++ 1 MB. *Scope control comes
  from a single authority, not from a shared specification.*
- **PLEAC** — GFDL again, and thin outside Perl/Python/Ruby/OCaml/Groovy.
- **Benchmarks Game**, **Programming-Language-Benchmarks** — speed-tuned code.
- **c2rust output** as a C→Rust proxy — transpiler goo wrapped in `unsafe`;
  measures the transpiler.
- **TheAlgorithms** — covers all 16 including Zig, but content volume runs 130×
  apart with no shared test suite. A fourth corpus at the same control level and
  size regime as one already held.
- **roc** (Rust→Zig) as a port measurement — a reimplementation, not a port. 45
  of ~300 file stems overlap, 6 of ~37 components share a name, and Zig-only
  components are most of the Zig tree.

A structural note for anyone looking again: **Zig is the binding constraint.**
Only pedagogical multi-language projects reach it. Anything drawn from
production ecosystems, or predating roughly 2020, will not have it.

## Reproducing

```sh
git clone https://github.com/exercism/{c,cpp,csharp,go,java,javascript,kotlin,php,python,ruby,rust,scala,swift,typescript,zig,bash}
cargo run --manifest-path tools/verbosity/Cargo.toml --release -- --corpus <dir>
```

The corpus is recognized from its layout; `--source rosetta|exercism|mal` forces
it. Each run rewrites the checked-in table, so run the corpus you intend to ship
**last**.
