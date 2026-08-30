# entl

Typed facts about a codebase and the GitHub repository around it.

`entl` walks a local checkout once and returns reusable facts rather than
verdicts: files and their language evidence, packages and workspaces across
Cargo and the JavaScript ecosystems, project boundaries, tool profiles and
task classifications, dependencies, artifacts, and recoverable diagnostics.
Alongside them it reads the repository's surroundings — workflows, Dependabot
and CODEOWNERS configuration, Action pinning, and repository settings.

```rust
use entl::codebase::{InventoryOptions, inspect};

let codebase = inspect(".", &InventoryOptions::default())?;
for package in &codebase.packages {
    println!("{} at {}", package.id, package.root.display());
}
# Ok::<(), entl::codebase::Error>(())
```

Two module trees, one crate. [`codebase`](src/codebase) answers what the source
tree contains; [`github`](src/github) answers what surrounds it, deriving its
facts from an inventory `codebase` already produced.

Entl reports observed facts and the evidence behind them. Policy, findings, and
remediation belong to its consumers — see
[ordnung](https://github.com/PowderworksCode/ordnung) for one.

Full documentation is in [the repository](https://github.com/PowderworksCode/entl).
MIT licensed.
