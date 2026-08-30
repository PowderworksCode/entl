# Third-party notices

Entl's own source is licensed under the MIT License; see [LICENSE](LICENSE).

This repository additionally redistributes prebuilt Tree-sitter grammars as
WebAssembly artifacts under `parser-packs/`. Each `grammar.wasm` is compiled
from the pinned upstream revision recorded in the adjacent `parser.toml`, whose
`sha256` field is the checksum of the artifact as distributed here. The
grammars are third-party works and remain under their own licenses, reproduced
in full below.

| Pack | Upstream | Revision | Version | License |
| --- | --- | --- | --- | --- |
| `parser-packs/rust` | [tree-sitter/tree-sitter-rust](https://github.com/tree-sitter/tree-sitter-rust) | `77a3747266f4d621d0757825e6b11edcbf991ca5` | 0.24.2 | MIT |
| `parser-packs/javascript` | [tree-sitter/tree-sitter-javascript](https://github.com/tree-sitter/tree-sitter-javascript) | `44c892e0be055ac465d5eeddae6d3e194424e7de` | 0.25.0 | MIT |
| `parser-packs/typescript` | [tree-sitter/tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) | `f975a621f4e7f532fe322e13c4f79495e0a7b2e7` | 0.23.2 | MIT |
| `parser-packs/tsx` | [tree-sitter/tree-sitter-typescript](https://github.com/tree-sitter/tree-sitter-typescript) | `f975a621f4e7f532fe322e13c4f79495e0a7b2e7` | 0.23.2 | MIT |

The `parser.toml` manifests and the `queries/` files under `parser-packs/` are
Entl's own work and fall under Entl's license.

## Verbosity corpora

The verbosity numbers in `crates/entl/src/codebase/profiles/verbosity.rs` and
`docs/verbosity-*.md` are measured by `tools/verbosity` from one of three
corpora, none of which is redistributed here.

### Exercism

The checked-in table and `docs/verbosity-exercism.md` are measured from the
[Exercism](https://github.com/exercism) track repositories, which are MIT
licensed. Exercism content is not redistributed here regardless; the tool reads
a checkout the operator downloads.

### mal

`docs/verbosity-mal.md` is measured from [mal](https://github.com/kanaka/mal),
which is licensed under the
[Mozilla Public License 2.0](https://www.mozilla.org/MPL/2.0/). It is published
as a mid-sized cross-check on the shipped numbers; no mal measurement is checked
into the crate.

### Rosetta Code

`docs/verbosity-rosetta.md` is derived from [Rosetta Code](https://rosettacode.org),
read through the [Rosetta Code Data](https://github.com/acmeism/RosettaCodeData)
mirror by `tools/verbosity`. It is published as a cross-check on the shipped
numbers; no Rosetta Code measurement is checked into the crate.

Rosetta Code content is licensed under the
[GNU Free Documentation License 1.2](https://www.gnu.org/licenses/old-licenses/fdl-1.2.en.html),
which [its copyright page](https://rosettacode.org/wiki/Rosetta_Code:Copyrights)
notes "is not compatible with most software licenses, including OSI-approved
licenses such as the GPL". No Rosetta Code content is redistributed here, and
that constraint is why the tool never copies corpus content from either source.
What it writes back is statistics — unit counts, size ratios, and a fitted
index. Those measurements are Entl's own work and fall under Entl's license.

## tree-sitter-rust

```
The MIT License (MIT)

Copyright (c) 2017 Maxim Sokolov

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## tree-sitter-javascript

```
The MIT License (MIT)

Copyright (c) 2014 Max Brunsfeld

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## tree-sitter-typescript

Covers both the `typescript` and `tsx` packs.

```
The MIT License (MIT)

Copyright (c) 2017 Max Brunsfeld

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
