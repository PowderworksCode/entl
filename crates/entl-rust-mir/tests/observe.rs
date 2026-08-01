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
#[test]
fn one_call_resolves_the_same_however_it_is_written() {
    let observations = observe("imports.rs");
    for spelling in ["qualified", "via_module"] {
        assert_eq!(
            callees(&observations, spelling),
            ["std::fs::read"],
            "`{spelling}` should resolve to the same definition as every other spelling"
        );
    }
    assert_eq!(
        callees(&observations, "via_item"),
        ["std::fs::read_to_string"]
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
