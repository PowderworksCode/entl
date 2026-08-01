# rosetta-verbosity

Measures how much source text each of Entl's profiled languages needs for the
same task, and regenerates:

- `crates/entl-codebase/src/profiles/verbosity.rs` — the checked-in table
- `docs/verbosity.md` — the method, the full matrix, and the caveats

## Running it

```sh
git clone https://github.com/acmeism/RosettaCodeData
cargo run --manifest-path tools/rosetta-verbosity/Cargo.toml --release -- \
  --corpus ../RosettaCodeData
```

`--baseline <language>` changes the language every index is expressed relative
to (default `c`). `--root <path>` changes where the outputs are written.

The corpus checkout is about 800 MB and is not vendored, submoduled, or
referenced by any build. Nothing in Entl needs this tool to build or test; it
runs by hand when the corpus is worth re-measuring.

## Licensing

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
src/measure.rs  strips comments using each language's own CommentSyntax
src/corpus.rs   reads the corpus and reduces each task to its median solution
src/stats.rs    pairwise ratios, the least-squares index, transitivity checks
src/emit.rs     renders the generated table and the report
```

`measure.rs` deliberately drives off `entl-codebase`'s `comment_syntax`. A
second, private copy of every language's comment rules would drift from the
profiles the numbers claim to describe.

## Why the matrix and not just the index

Every pair is measured only on tasks both languages implement, and no two pairs
share the same task set. That makes the ratios non-transitive: C-to-Java is not
(C-to-Python)(Python-to-Java). The index is the least-squares reconciliation of
all pairs; `verbosity_ratio` returns the pair as actually measured. Use the
ratio when comparing two specific languages, and the index when ranking many.
