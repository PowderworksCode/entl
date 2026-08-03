//! Span-anchored semantic observations about a codebase.
//!
//! Compilers disagree about what they hold in memory. Some expose a control
//! flow graph, some a typed syntax tree, some neither. What they all can answer
//! is a question about a place in the source: what does this name refer to,
//! what type does this expression have, where does this call go. Those answers
//! are what this schema records, so that a consumer asks the same questions of
//! every language and each language answers with whatever its tooling knows.
//!
//! Nothing here is an intermediate representation. Unifying a control flow
//! graph with a typed syntax tree yields something less useful than either;
//! unifying the answers they can give does not.
//!
//! Every observation is optional. A language with no type information produces
//! no types, and a consumer must be able to tell that apart from a language
//! that produced none because there were none to find. [`Coverage`] records
//! which questions a provider actually attempted.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const SEMANTIC_OBSERVATION_SCHEMA: u32 = 1;

/// A stable identifier for something a program defines.
///
/// Each language mints these its own way — a Rust definition path, a
/// TypeScript file and qualified name — but within one observation set they
/// are comparable, and that is what lets a reference be tied to a definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(pub String);

impl EntityId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A region of source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Span {
    pub path: PathBuf,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// What kind of thing a definition is.
///
/// Deliberately coarse. The distinctions that survive translation between
/// languages are few, and a consumer that needs more should look at the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntityKind {
    Function,
    Method,
    Type,
    Interface,
    Constant,
    Field,
    Module,
    Other,
}

/// Whether a definition is reachable from outside the thing that defines it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Public,
    Crate,
    Private,
    Unknown,
}

/// Something the program defines.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Definition {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    /// The type, trait, or module this is written inside.
    pub container: Option<EntityId>,
    pub visibility: Visibility,
    /// Absent when a definition has no source of its own, as with a compiler
    /// generated item or one that came from another crate.
    pub span: Option<Span>,
}

/// A place in the source that names something defined elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Reference {
    pub span: Span,
    pub resolves_to: EntityId,
}

/// How a call's destination was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dispatch {
    /// One destination, known for certain.
    Static,
    /// Chosen at run time from the candidates given.
    Virtual,
    /// A destination named before the generic arguments were known.
    ///
    /// The call was read from a body the compiler had not instantiated, so the
    /// name is the one the source wrote rather than the one that will run. It
    /// is a real edge and belongs in the graph — a generic function's calls are
    /// otherwise absent entirely — but it is NOT one destination known for
    /// certain, and a consumer that treats it as such will be confidently
    /// wrong. Every `T::clone` in a library reads as `Clone::clone` here, and
    /// cloning an `Arc` and cloning a `String` are not the same behavior.
    Unmonomorphized,
    /// The provider found a call but could not say where it goes.
    Unknown,
}

/// A call, and where it may go.
///
/// `to` is a list rather than one destination on purpose. A monomorphized
/// Rust call has exactly one; a TypeScript method call may have several; an
/// unresolved call has none. A schema that assumed one would have to break to
/// admit the second language.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CallEdge {
    pub span: Span,
    /// The definition the call is written inside.
    pub from: EntityId,
    pub to: Vec<EntityId>,
    pub dispatch: Dispatch,
}

/// A type, named as shallowly as is portable.
///
/// Rust traits and TypeScript structural types cannot be reconciled, and this
/// does not try. It records what the type is called, what it was applied to,
/// and how the language itself would print it. That is enough to tell a
/// `Vec` from a `HashMap`, which is what callers actually ask.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeRef {
    /// The type constructor's name, without arguments: `Vec`, `HashMap`.
    pub head: String,
    pub arguments: Vec<TypeRef>,
    /// The language's own rendering, for display and exact comparison.
    pub display: String,
}

/// The type of the expression at a place in the source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeAt {
    pub span: Span,
    pub type_ref: TypeRef,
}

/// A type satisfies an interface, trait, or protocol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Implements {
    pub type_id: EntityId,
    pub interface: EntityId,
    /// Where the implementation is written, when it is written anywhere.
    pub span: Option<Span>,
}

/// Which questions a provider attempted.
///
/// Absence of an observation is ambiguous on its own: a call with no
/// destination may be unresolvable, or the provider may not resolve calls at
/// all. Recording what was attempted is what lets a consumer distinguish
/// "nothing to report" from "not looked at", and report incompleteness
/// honestly rather than silently under-reporting.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Coverage {
    pub definitions: bool,
    pub references: bool,
    pub call_edges: bool,
    pub types: bool,
    pub implements: bool,
}

/// Something a provider could not observe.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Gap {
    pub span: Option<Span>,
    pub message: String,
}

/// How a set of observations was produced.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Provenance {
    /// The provider that produced these, such as `rust.mir`.
    pub provider: String,
    pub provider_version: String,
    /// The toolchain the provider ran, so observations can be invalidated when
    /// the compiler changes and not only when the source does.
    pub toolchain: String,
    /// The unit observed: a crate, a package, a project.
    pub unit: String,
}

/// Everything one provider observed about one unit of source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticObservations {
    pub schema: u32,
    pub provenance: Provenance,
    pub coverage: Coverage,
    #[serde(default)]
    pub definitions: Vec<Definition>,
    #[serde(default)]
    pub references: Vec<Reference>,
    #[serde(default)]
    pub call_edges: Vec<CallEdge>,
    #[serde(default)]
    pub types: Vec<TypeAt>,
    #[serde(default)]
    pub implements: Vec<Implements>,
    #[serde(default)]
    pub gaps: Vec<Gap>,
}

impl SemanticObservations {
    pub fn new(provenance: Provenance) -> Self {
        Self {
            schema: SEMANTIC_OBSERVATION_SCHEMA,
            provenance,
            coverage: Coverage::default(),
            definitions: Vec::new(),
            references: Vec::new(),
            call_edges: Vec::new(),
            types: Vec::new(),
            implements: Vec::new(),
            gaps: Vec::new(),
        }
    }

    /// Sort every collection so that the same source and toolchain produce
    /// byte-identical output, which is what makes these cacheable.
    pub fn canonicalize(&mut self) {
        self.definitions.sort();
        self.definitions.dedup();
        self.references.sort();
        self.references.dedup();
        self.call_edges.sort();
        self.call_edges.dedup();
        self.types.sort();
        self.types.dedup();
        self.implements.sort();
        self.implements.dedup();
        self.gaps.sort();
        self.gaps.dedup();
    }

    /// The definitions a call at `span` may reach.
    pub fn callees(&self, span: &Span) -> &[EntityId] {
        self.call_edges
            .iter()
            .find(|edge| &edge.span == span)
            .map_or(&[], |edge| edge.to.as_slice())
    }

    /// Every call made from within one definition.
    pub fn calls_from<'a>(&'a self, from: &'a EntityId) -> impl Iterator<Item = &'a CallEdge> {
        self.call_edges
            .iter()
            .filter(move |edge| &edge.from == from)
    }

    pub fn definition(&self, id: &EntityId) -> Option<&Definition> {
        self.definitions.iter().find(|entity| &entity.id == id)
    }

    /// Re-express every span relative to `root`, dropping what falls outside it.
    ///
    /// A provider records paths as the compiler saw them, from wherever the
    /// build ran. A consumer scanning a subdirectory needs them relative to
    /// that. Observations about code outside the scan are discarded rather
    /// than rebased, because a scan should not report on what it was not asked
    /// to look at.
    pub fn rebase(&mut self, root: &std::path::Path) {
        if root.as_os_str().is_empty() || root == std::path::Path::new(".") {
            return;
        }
        fn under(path: &std::path::Path, root: &std::path::Path) -> Option<PathBuf> {
            path.strip_prefix(root).ok().map(PathBuf::from)
        }
        self.definitions
            .retain_mut(|definition| match &definition.span {
                Some(span) => match under(&span.path, root) {
                    Some(path) => {
                        if let Some(span) = definition.span.as_mut() {
                            span.path = path;
                        }
                        true
                    }
                    None => false,
                },
                // a definition with no span is not tied to a place, so it survives
                None => true,
            });
        self.references
            .retain_mut(|reference| match under(&reference.span.path, root) {
                Some(path) => {
                    reference.span.path = path;
                    true
                }
                None => false,
            });
        self.call_edges
            .retain_mut(|edge| match under(&edge.span.path, root) {
                Some(path) => {
                    edge.span.path = path;
                    true
                }
                None => false,
            });
        self.types
            .retain_mut(|observed| match under(&observed.span.path, root) {
                Some(path) => {
                    observed.span.path = path;
                    true
                }
                None => false,
            });
        self.canonicalize();
    }

    /// Combine observations of several units into one.
    ///
    /// Providers observe one compilation unit at a time, but a call graph does
    /// not stop at a crate boundary: a call from one unit into another is only
    /// a resolvable edge once both are present. Returns `None` for an empty
    /// input, because no observations and observations of nothing are different
    /// claims.
    pub fn merge(units: impl IntoIterator<Item = Self>, unit: impl Into<String>) -> Option<Self> {
        let mut units = units.into_iter();
        let first = units.next()?;
        let mut merged = Self {
            provenance: Provenance {
                unit: unit.into(),
                ..first.provenance.clone()
            },
            ..first
        };
        for other in units {
            // a question is only covered when every unit attempted it, or the
            // merged set would claim coverage it does not have
            merged.coverage.definitions &= other.coverage.definitions;
            merged.coverage.references &= other.coverage.references;
            merged.coverage.call_edges &= other.coverage.call_edges;
            merged.coverage.types &= other.coverage.types;
            merged.coverage.implements &= other.coverage.implements;
            if other.provenance.toolchain != merged.provenance.toolchain {
                merged.gaps.push(Gap {
                    span: None,
                    message: format!(
                        "{} was observed by {} but {} by {}",
                        other.provenance.unit,
                        other.provenance.toolchain,
                        merged.provenance.unit,
                        merged.provenance.toolchain
                    ),
                });
            }
            merged.definitions.extend(other.definitions);
            merged.references.extend(other.references);
            merged.call_edges.extend(other.call_edges);
            merged.types.extend(other.types);
            merged.implements.extend(other.implements);
            merged.gaps.extend(other.gaps);
        }
        merged.canonicalize();
        Some(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let merged = SemanticObservations::merge([observations(), other], "workspace")
            .expect("units to merge");
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
}
