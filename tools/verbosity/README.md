# verbosity

Measures how much source text each of Entl's profiled languages needs for the
same task, and regenerates:

- `crates/entl/src/codebase/profiles/verbosity.rs` — the checked-in table
- `docs/verbosity-<corpus>.md` — the method, the full matrix, and the caveats

## The two corpora

**Rosetta Code** — ~1,750 tasks, solutions contributed by anyone. Large, but
uncontrolled: two entries for one task do not always answer the same question,
so each (language, task) pair is reduced to its median solution.

**Exercism** — ~165 practice exercises per track, each with exactly one
reference solution named by `files.example` in the exercise's
`.meta/config.json`, written against a shared specification and verified by a
shared test suite. Smaller, but nothing needs averaging.

**mal** — one Lisp interpreter of one to five thousand lines, implemented in
each of 80-odd languages against the same eleven-step guide and the same test
suite. The only mid-sized program available in all of Entl's languages with its
scope pinned. It contributes exactly one unit per language, so it has no spread
to put an error bar on and cannot drive the shipped table; the tool omits the
transitivity and balanced-panel sections for it rather than reporting arithmetic
as agreement.

They rank the languages alike and disagree on magnitude. Keeping all three is
the point: an index they agree on is a fact about the languages, one they
disagree on is a fact about the corpora, and the Exercism-to-mal gap is a fact
about program size — the languages spread 2.5x apart on small exercises and 4.9x
apart on a mid-sized program.

## Running it

```sh
git clone https://github.com/acmeism/RosettaCodeData
cargo run --manifest-path tools/verbosity/Cargo.toml --release -- \
  --corpus ../RosettaCodeData
```

```sh
git clone https://github.com/kanaka/mal
cargo run --manifest-path tools/verbosity/Cargo.toml --release -- --corpus ../mal
```

```sh
mkdir exercism && cd exercism
for t in c cpp csharp go java javascript kotlin php python ruby rust scala \
         swift typescript zig bash; do
  git clone --depth 1 https://github.com/exercism/$t $t
done
cd .. && cargo run --manifest-path tools/verbosity/Cargo.toml --release -- \
  --corpus exercism
```

The corpus is recognized from its layout; `--source rosetta|exercism` forces it.
`--baseline <language>` changes the language every index is expressed relative
to (default `c`). `--root <path>` changes where the outputs are written.

Neither corpus is vendored, submoduled, or referenced by any build. Nothing in
Entl needs this tool to build or test; it runs by hand when a corpus is worth
re-measuring.

## Licensing

Exercism's track repositories are MIT and mal is MPL-2.0. Rosetta Code is
neither.

Rosetta Code content is licensed under the GNU Free Documentation License 1.2.
[Rosetta Code's own copyright page](https://rosettacode.org/wiki/Rosetta_Code:Copyrights)
states that the material "is not compatible with most software licenses,
including OSI-approved licenses such as the GPL" — which includes Entl's MIT
license.

So no corpus source is copied into this repository, quoted in a test fixture, or
embedded in a doc example. What crosses the boundary is measurements: counts,
ratios, and a fitted index. Facts about a work are not the work. If that
boundary ever needs to move — a fixture with a real snippet, a quoted example in
the docs — it needs a licensing decision first, not a convenient copy.

## Layout

```text
src/measure.rs         strips comments using each language's own CommentSyntax
src/corpus/mod.rs      what a corpus is, and which one a checkout holds
src/corpus/rosetta.rs  Rosetta Code: median over contributed solutions
src/corpus/exercism.rs Exercism: the reference solution named by each exercise
src/corpus/mal.rs      mal: one whole interpreter per language, one unit
src/stats.rs           pairwise ratios, least-squares index, transitivity checks
src/emit.rs            renders the generated table and the report
```

`measure.rs` deliberately drives off `entl`'s `comment_syntax`. A
second, private copy of every language's comment rules would drift from the
profiles the numbers claim to describe.

## Background

[notes/verbosity.md](../../notes/verbosity.md) records what these numbers turned
out to be worth: the corpora rank the languages alike and disagree on magnitude
by up to a factor of two, the spread roughly doubles on a mid-sized program, and
this measures writing-from-a-spec rather than porting. It also lists the corpora
that were evaluated and rejected, so the next search can start further along.

## Why the matrix and not just the index

Every pair is measured only on tasks both languages implement, and no two pairs
share the same task set. That makes the ratios non-transitive: C-to-Java is not
(C-to-Python)(Python-to-Java). The index is the least-squares reconciliation of
all pairs; `verbosity_ratio` returns the pair as actually measured. Use the
ratio when comparing two specific languages, and the index when ranking many.
