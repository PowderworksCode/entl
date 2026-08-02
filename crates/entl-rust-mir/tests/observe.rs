#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The driver's contract: a call resolves to the same definition however it is
//! written, and what was not attempted is reported rather than implied.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use entl_semantics::{Dispatch, EntityId, SemanticObservations};

fn observe(fixture: &str) -> SemanticObservations {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = tempdir();
    let status = Command::new(env!("CARGO_BIN_EXE_entl-rust-mir"))
        .env("ENTL_RUST_MIR_OUTPUT", &output)
        .args([
            "--crate-type",
            "lib",
            "--crate-name",
            "fixture",
            "--edition",
            "2021",
        ])
        .arg(crate_root.join("tests/fixtures").join(fixture))
        .arg("--out-dir")
        .arg(&output)
        .status()
        .expect("running the driver");
    assert!(status.success(), "driver failed on {fixture}");
    let observations = std::fs::read(output.join("fixture.json")).expect("observations written");
    serde_json::from_slice(&observations).expect("observations parse")
}

/// A directory of its own for every invocation.
///
/// Tests run in parallel and each one drives a whole compilation, so sharing an
/// output directory means concurrent runs overwrite each other's observations.
fn tempdir() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "entl-rust-mir-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).expect("temporary directory");
    path
}

fn callees(observations: &SemanticObservations, from: &str) -> Vec<String> {
    let from = EntityId::new(format!("fixture::{from}"));
    observations
        .calls_from(&from)
        .flat_map(|edge| edge.to.iter())
        .map(|to| to.as_str().to_owned())
        .collect()
}

/// The point of the whole exercise: how a call is written stops mattering.
///
/// The destination is a monomorphized instance, so a generic function carries
/// the arguments it was called with. A consumer matching against a catalog of
/// declared paths has to normalize them away; what matters here is that every
/// spelling arrives at one name.
#[test]
fn one_call_resolves_the_same_however_it_is_written() {
    let observations = observe("imports.rs");
    for spelling in ["qualified", "via_module"] {
        assert_eq!(
            callees(&observations, spelling),
            ["std::fs::read::<&str>"],
            "`{spelling}` should resolve to the same definition as every other spelling"
        );
    }
    assert_eq!(
        callees(&observations, "via_item"),
        ["std::fs::read_to_string::<&str>"]
    );
}

/// A trait method is not one destination, and resolution is what says so.
///
/// Cloning an `Arc` bumps a count and cloning a `String` copies a buffer. Both
/// are written `.clone()` and both are `std::clone::Clone::clone` until the
/// instance is resolved, so a consumer asking what a call costs gets nothing
/// from the unresolved name.
#[test]
fn a_trait_method_carries_the_type_it_was_called_on() {
    let observations = observe("receivers.rs");
    for (function, expected) in [
        ("clone_an_arc", "std::sync::Arc<std::string::String>"),
        ("clone_a_string", "std::string::String"),
        ("clone_a_vec", "std::vec::Vec<u8>"),
    ] {
        let resolved = callees(&observations, function);
        assert_eq!(resolved.len(), 1, "{function}: {resolved:?}");
        assert!(
            resolved[0].contains(expected),
            "{function} should name {expected}, got {}",
            resolved[0]
        );
    }
    // the container being built is part of what `collect` does
    assert!(
        callees(&observations, "collected")
            .iter()
            .any(|callee| callee.contains("collect::<std::vec::Vec<u8>>")),
        "{:?}",
        callees(&observations, "collected")
    );
}

/// Local edges are what let an effect propagate to a caller.
#[test]
fn calls_between_local_definitions_are_recorded() {
    let observations = observe("imports.rs");
    assert_eq!(
        callees(&observations, "local_caller"),
        ["fixture::via_module"]
    );
}

#[test]
fn resolved_calls_are_reported_as_static_dispatch() {
    let observations = observe("imports.rs");
    let resolved = observations
        .call_edges
        .iter()
        .filter(|edge| edge.dispatch == Dispatch::Static)
        .count();
    assert_eq!(resolved, observations.call_edges.len());
}

/// Absence has to be distinguishable from never having looked.
#[test]
fn coverage_states_which_questions_were_attempted() {
    let observations = observe("imports.rs");
    assert!(observations.coverage.call_edges);
    assert!(observations.coverage.definitions);
    assert!(
        !observations.coverage.types,
        "types are not observed yet, and the schema should say so"
    );
    assert!(observations.types.is_empty());
}

#[test]
fn observations_carry_the_compiler_that_produced_them() {
    let observations = observe("imports.rs");
    assert_eq!(observations.provenance.provider, "rust.mir");
    assert!(
        observations.provenance.toolchain.contains("rustc"),
        "{:?}",
        observations.provenance.toolchain
    );
    assert_eq!(observations.provenance.unit, "fixture");
}

/// A build tool asks the compiler what it is before asking it to compile.
///
/// `rustc -vV` and `rustc --print` compile nothing, and `run!` reports that as
/// `CompilerError::Skipped`. Treating every error as failure made those probes
/// exit non-zero, so `RUSTC=entl-rust-mir cargo build` died before reaching the
/// first crate — which is why there was no way to produce observations from an
/// ordinary build.
#[test]
fn a_probe_that_compiles_nothing_still_succeeds() {
    for probe in [vec!["-vV"], vec!["--print", "sysroot"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_entl-rust-mir"))
            .args(&probe)
            .output()
            .expect("running the driver");
        assert!(
            output.status.success(),
            "{probe:?} exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "{probe:?} answered nothing on stdout"
        );
    }
}
