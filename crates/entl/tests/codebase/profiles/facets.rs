// Tests for `src/codebase/profiles/facets.rs`.
//
// A facet is what several languages have in common — that they host components,
// or style, or are structured code. Languages point at these statics, so an
// unregistered one is a facet no language can be asked about.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{COMPONENT_HOST, STRUCTURED_CODE, STYLE_HOST, language_facet};

#[test]
fn every_declared_facet_is_reachable_by_its_id() {
    for facet in [&STRUCTURED_CODE, &COMPONENT_HOST, &STYLE_HOST] {
        let found = language_facet(facet.id).expect("the facet is registered");
        assert_eq!(found.id, facet.id);
    }
}
