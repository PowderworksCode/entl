use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{PackageId, PackageLanguage};
use langbank::{EcosystemId, ProjectFacetId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub root: PathBuf,
    pub packages: Vec<PackageId>,
    pub languages: Vec<PackageLanguage>,
    pub ecosystems: BTreeSet<EcosystemId>,
    pub facets: BTreeSet<ProjectFacetId>,
    pub evidence: BTreeSet<PathBuf>,
}

impl Project {
    pub fn has_language(&self, language: &str) -> bool {
        self.languages
            .iter()
            .any(|candidate| candidate.language.as_str() == language)
    }

    pub fn uses_ecosystem(&self, ecosystem: &str) -> bool {
        self.ecosystems
            .iter()
            .any(|candidate| candidate.as_str() == ecosystem)
    }

    pub fn has_facet(&self, facet: &str) -> bool {
        self.facets
            .iter()
            .any(|candidate| candidate.as_str() == facet)
    }
}
