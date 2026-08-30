//! Typed facts about a codebase and the GitHub repository around it.
//!
//! The two halves are separate module trees because they answer different
//! questions from different evidence. [`codebase`] reads a source tree into
//! files, languages, packages, projects, and workspaces. [`github`] reads the
//! workflows, manifests, and settings that surround one, and derives its facts
//! from an inventory [`codebase`] already produced.
//!
//! They ship as one crate because a consumer that wants repository facts wants
//! both, and versioning them apart bought nothing but the chance to skew.

pub mod codebase;
pub mod github;
