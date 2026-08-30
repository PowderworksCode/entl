use std::collections::BTreeSet;
use std::sync::LazyLock;

use super::registry;

/// A reusable fact about the source surfaces a language can contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageFacet {
    pub id: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageFacetRegistration(pub &'static LanguageFacet);

registry::collect!(LanguageFacetRegistration);

static REGISTERED: LazyLock<Vec<&'static LanguageFacet>> = LazyLock::new(|| {
    let mut facets = registry::iter::<LanguageFacetRegistration>
        .into_iter()
        .map(|registration| registration.0)
        .collect::<Vec<_>>();
    facets.sort_by_key(|facet| facet.id);
    let mut ids = BTreeSet::new();
    for facet in &facets {
        assert!(ids.insert(facet.id), "duplicate language facet ID");
    }
    facets
});

pub fn language_facets() -> &'static [&'static LanguageFacet] {
    REGISTERED.as_slice()
}

pub fn language_facet(id: &str) -> Option<&'static LanguageFacet> {
    language_facets()
        .binary_search_by_key(&id, |facet| facet.id)
        .ok()
        .map(|index| language_facets()[index])
}
