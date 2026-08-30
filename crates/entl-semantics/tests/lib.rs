// Tests for `src/lib.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::PathBuf;

use entl_semantics::*;

fn span(path: &str, line: u32) -> Span {
    Span {
        path: PathBuf::from(path),
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: 10,
    }
}

fn observations() -> SemanticObservations {
    let mut observed = SemanticObservations::new(Provenance {
        provider: "rust.mir".to_owned(),
        provider_version: "0.0.0".to_owned(),
        toolchain: "nightly-test".to_owned(),
        unit: "fixture".to_owned(),
    });
    observed.coverage.call_edges = true;
    observed.call_edges.push(CallEdge {
        span: span("src/lib.rs", 4),
        from: EntityId::new("fixture::read_config"),
        to: vec![EntityId::new("std::fs::read")],
        dispatch: Dispatch::Static,
    });
    observed.call_edges.push(CallEdge {
        span: span("src/lib.rs", 9),
        from: EntityId::new("fixture::load"),
        to: vec![
            EntityId::new("fixture::Disk::open"),
            EntityId::new("fixture::Memory::open"),
        ],
        dispatch: Dispatch::Virtual,
    });
    observed
}

#[test]
fn a_call_records_every_destination_it_may_reach() {
    let observed = observations();
    assert_eq!(
        observed.callees(&span("src/lib.rs", 4)),
        [EntityId::new("std::fs::read")]
    );
    assert_eq!(observed.callees(&span("src/lib.rs", 9)).len(), 2);
    // a span with no call is not a call with no destinations
    assert!(observed.callees(&span("src/lib.rs", 99)).is_empty());
}

#[test]
fn calls_are_grouped_by_the_definition_that_makes_them() {
    let observed = observations();
    let from = EntityId::new("fixture::read_config");
    assert_eq!(observed.calls_from(&from).count(), 1);
    assert_eq!(observed.calls_from(&EntityId::new("nobody")).count(), 0);
}

#[test]
fn canonicalizing_makes_output_independent_of_discovery_order() {
    let mut forward = observations();
    let mut backward = observations();
    backward.call_edges.reverse();
    forward.canonicalize();
    backward.canonicalize();
    assert_eq!(forward, backward);
}

#[test]
fn coverage_separates_nothing_found_from_nothing_attempted() {
    let observed = observations();
    assert!(observed.coverage.call_edges, "calls were attempted");
    assert!(
        !observed.coverage.types,
        "types were not attempted, so having none says nothing"
    );
}

#[test]
fn merging_units_joins_their_call_graphs() {
    let mut other = SemanticObservations::new(Provenance {
        provider: "rust.mir".to_owned(),
        provider_version: "0.0.0".to_owned(),
        toolchain: "nightly-test".to_owned(),
        unit: "other".to_owned(),
    });
    other.coverage.call_edges = true;
    other.coverage.types = true;
    other.call_edges.push(CallEdge {
        span: span("other/src/lib.rs", 2),
        from: EntityId::new("other::entry"),
        to: vec![EntityId::new("fixture::read_config")],
        dispatch: Dispatch::Static,
    });

    let merged =
        SemanticObservations::merge([observations(), other], "workspace").expect("units to merge");
    assert_eq!(merged.provenance.unit, "workspace");
    assert_eq!(merged.call_edges.len(), 3);
    // the cross-unit edge is only resolvable once both units are present
    assert_eq!(
        merged.callees(&span("other/src/lib.rs", 2)),
        [EntityId::new("fixture::read_config")]
    );
    assert!(merged.coverage.call_edges, "both units attempted calls");
    assert!(
        !merged.coverage.types,
        "one unit did not attempt types, so the merge cannot claim them"
    );
}

#[test]
fn rebasing_relates_observations_to_the_directory_being_scanned() {
    let mut observed = observations();
    observed.call_edges.push(CallEdge {
        span: span("tests/helper.rs", 3),
        from: EntityId::new("fixture::test_only"),
        to: vec![EntityId::new("std::fs::read")],
        dispatch: Dispatch::Static,
    });
    observed.rebase(std::path::Path::new("src"));

    let paths = observed
        .call_edges
        .iter()
        .map(|edge| edge.span.path.clone())
        .collect::<Vec<_>>();
    assert!(
        paths
            .iter()
            .all(|path| path == std::path::Path::new("lib.rs")),
        "{paths:?}"
    );
    assert_eq!(
        paths.len(),
        2,
        "the edge outside the scanned directory is dropped, not rebased"
    );
}

#[test]
fn rebasing_on_the_current_directory_changes_nothing() {
    let mut observed = observations();
    let before = observed.clone();
    observed.rebase(std::path::Path::new("."));
    assert_eq!(observed, before);
}

#[test]
fn merging_nothing_is_not_the_same_as_observing_nothing() {
    assert!(SemanticObservations::merge([], "empty").is_none());
}

#[test]
fn merging_units_from_different_toolchains_records_a_gap() {
    let mut other = observations();
    other.provenance.toolchain = "nightly-other".to_owned();
    other.provenance.unit = "other".to_owned();
    let merged = SemanticObservations::merge([observations(), other], "workspace").unwrap();
    assert!(
        merged
            .gaps
            .iter()
            .any(|gap| gap.message.contains("nightly-other")),
        "{:?}",
        merged.gaps
    );
}

#[test]
fn observations_round_trip_through_json() {
    let mut observed = observations();
    observed.canonicalize();
    let encoded = serde_json::to_string(&observed).unwrap();
    let decoded: SemanticObservations = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, observed);
    assert_eq!(decoded.schema, SEMANTIC_OBSERVATION_SCHEMA);
}
