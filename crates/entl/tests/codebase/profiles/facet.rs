// Tests for `src/codebase/profiles/facet.rs`: the facet registry.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{language_facet, language_facets};

/// Sorted, because the lookup binary-searches it. An unsorted registry does not
/// fail — it silently fails to find things.
#[test]
fn the_registry_is_sorted_and_every_entry_is_findable() {
    let facets = language_facets();
    assert!(!facets.is_empty());
    assert!(facets.windows(2).all(|pair| pair[0].id < pair[1].id));
    for facet in facets {
        assert_eq!(
            language_facet(facet.id).map(|found| found.id),
            Some(facet.id)
        );
    }
}

#[test]
fn an_unknown_id_is_none_rather_than_a_panic() {
    assert!(language_facet("no-such-facet").is_none());
}
