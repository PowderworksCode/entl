use std::path::PathBuf;

use entl_codebase::{
    DiscoveryBuilder, DiscoveryHandler, DiscoveryHandlerRegistration, DiscoveryPhase,
    InventoryOptions, discovery_handlers, discovery_registry, inspect,
};

fn add_fixture_facet(builder: &mut DiscoveryBuilder<'_>) {
    builder.add_project_facet(
        PathBuf::new(),
        "fixture-enrichment",
        [PathBuf::from("fixture.signal")],
    );
}

static FIXTURE_HANDLER: DiscoveryHandler = DiscoveryHandler {
    id: "fixture.enrichment",
    phase: DiscoveryPhase::Enrichment,
    run: add_fixture_facet,
};

discovery_registry::submit! {
    DiscoveryHandlerRegistration(&FIXTURE_HANDLER)
}

#[test]
fn downstream_discovery_handlers_enrich_projects_after_builtin_discovery() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("fixture.signal"), "").unwrap();

    let inventory = inspect(temp.path(), &InventoryOptions::default()).unwrap();
    assert!(
        inventory
            .project("")
            .unwrap()
            .has_facet("fixture-enrichment")
    );
    assert_eq!(
        discovery_handlers()
            .iter()
            .map(|handler| handler.id)
            .collect::<Vec<_>>(),
        [
            "entl.cargo-manifests",
            "entl.node-manifests",
            "entl.node-ecosystems",
            "entl.relationships",
            "entl.projects",
            "entl.artifacts",
            "fixture.enrichment"
        ]
    );
}
