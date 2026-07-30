use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{LanguageId, PackageId};
use crate::LanguageProfile;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LanguageEvidence {
    Extension { extension: String },
    Filename { filename: String },
    Shebang { interpreter: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageDetection {
    pub language: LanguageId,
    pub evidence: Vec<LanguageEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Codebase-relative path. The root itself is never a file entry.
    pub path: PathBuf,
    pub size: u64,
    pub language: Option<LanguageDetection>,
    /// The nearest owning package of each package kind.
    pub packages: Vec<PackageId>,
}

impl FileEntry {
    pub fn has_language(&self, language: &str) -> bool {
        self.language
            .as_ref()
            .is_some_and(|detection| detection.language.as_str() == language)
    }

    pub fn has_language_profile(&self, language: &LanguageProfile) -> bool {
        self.language
            .as_ref()
            .and_then(LanguageDetection::profile)
            .is_some_and(|candidate| std::ptr::eq(candidate, language))
    }
}
