//! What the checker can tell a consumer that syntax cannot.

use std::path::PathBuf;

use entl_ts_observe::{Options, available, observe};

fn typescript() -> Option<PathBuf> {
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tsprobe/node_modules/typescript/lib/typescript.js");
    candidate.is_file().then_some(candidate)
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/receivers")
}

fn options() -> Options {
    Options {
        typescript: typescript(),
        node: None,
    }
}

/// The observation that justifies running a compiler at all.
///
/// `xs.filter(p)[0]` and `jerk.filter(p)[0]` are the same text. One is a search
/// over an array and the other is a method someone wrote on their own type, and
/// no amount of syntax separates them.
#[test]
fn a_receiver_carries_the_type_it_actually_has() {
    if !available(&options()) || typescript().is_none() {
        eprintln!("skipping: no node or no TypeScript compiler on this machine");
        return;
    }
    let observed = observe(fixture(), &options()).expect("observe the fixture");

    assert!(observed.coverage.types, "types were attempted");
    assert!(observed.coverage.call_edges, "calls were attempted");

    let heads = observed
        .types
        .iter()
        .map(|observed| observed.type_ref.head.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        heads.contains("Array"),
        "an array receiver is an Array: {heads:?}"
    );
    assert!(
        heads.contains("JerkCode"),
        "a user-defined type is not an Array: {heads:?}"
    );
}

/// Spans are relative to the tree, or they cannot be joined to anything.
#[test]
fn every_span_is_relative_to_the_project() {
    if !available(&options()) || typescript().is_none() {
        return;
    }
    let observed = observe(fixture(), &options()).expect("observe the fixture");
    for observed in &observed.types {
        assert!(
            observed.span.path.is_relative(),
            "{} is not codebase-relative",
            observed.span.path.display()
        );
    }
}

/// Observing twice observes the same thing.
#[test]
fn observations_are_deterministic() {
    if !available(&options()) || typescript().is_none() {
        return;
    }
    let first = observe(fixture(), &options()).expect("observe");
    let second = observe(fixture(), &options()).expect("observe again");
    assert_eq!(first, second);
}

/// A machine that cannot observe says so rather than reporting nothing.
#[test]
fn a_missing_runtime_is_reported_rather_than_read_as_an_empty_project() {
    let broken = Options {
        typescript: typescript(),
        node: Some(PathBuf::from("definitely-not-a-node-on-this-machine")),
    };
    assert!(!available(&broken));
    assert!(observe(fixture(), &broken).is_err());
}
