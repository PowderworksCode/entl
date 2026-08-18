//! Identifiers for the things entl itself owns: packages and workspaces.
//!
//! Languages, artifacts, ecosystems and project facets are langbank's to name;
//! code that needs those identifiers takes `langbank::LanguageId` and friends
//! directly — entl does not re-export them. The macro here is private on
//! purpose: it mints entl's two ids and nothing else, and the convention it
//! encodes (ordered, hashable, serialised as a bare string) matches langbank's
//! so the two families read the same on disk.

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

string_id!(
    /// A package within a codebase, as entl inventories it.
    PackageId
);
string_id!(
    /// A workspace within a codebase, as entl inventories it.
    WorkspaceId
);
