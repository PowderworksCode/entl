#!/usr/bin/env bash
# Stand up a fresh Entl checkout: hooks, the workspace, and the two crates that
# sit outside it. Safe to re-run; every step is idempotent.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# A clone runs no hooks until it is pointed at them: core.hooksPath is per-clone
# configuration, so nothing a checkout carries can set it for you.
git config core.hooksPath .githooks
if [ ! -d .githooks ]; then
    echo "note: .githooks is fleet-managed and not synced here yet; git will"
    echo "      start using it the moment ordnung writes it."
fi

if ! command -v cargo >/dev/null; then
    echo "error: cargo is not on PATH; install Rust from https://rustup.rs" >&2
    exit 1
fi

# rust-toolchain.toml names the channel, and rustup installs it on first use.
echo "== workspace"
cargo build --workspace --all-targets

# Deliberately outside the workspace, so --workspace does not reach it: a data
# tool on the stable toolchain that regenerates the checked-in verbosity table.
echo "== tools/verbosity"
cargo build --manifest-path tools/verbosity/Cargo.toml

# entl-rust-mir replaces rustc for one compilation, so it needs the pinned
# nightly and the compiler's private crates -- roughly a gigabyte of toolchain.
# Building it is opt-in rather than the price of a first checkout.
mir_channel=$(sed -n 's/^channel = "\(.*\)"/\1/p' crates/entl-rust-mir/rust-toolchain.toml)
echo "== crates/entl-rust-mir"
if [ "${ENTL_DEV_RUST_MIR:-0}" = "1" ] ||
    rustup toolchain list 2>/dev/null | grep -q "^${mir_channel}"; then
    cargo build --manifest-path crates/entl-rust-mir/Cargo.toml
else
    echo "skipped: needs the ${mir_channel} toolchain."
    echo "build it with: ENTL_DEV_RUST_MIR=1 scripts/dev.sh"
fi

echo
echo "ready. the gate this repository runs in CI:"
echo "  cargo fmt --all --check"
echo "  cargo clippy --workspace --all-targets -- -D warnings"
echo "  cargo test --workspace"
