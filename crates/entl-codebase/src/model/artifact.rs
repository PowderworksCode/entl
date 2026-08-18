use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use langbank::ArtifactId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub profile: ArtifactId,
    pub root: PathBuf,
    pub evidence: BTreeSet<PathBuf>,
}
