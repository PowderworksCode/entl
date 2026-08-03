//! Observes resolved Rust semantics by running as the compiler.
//!
//! Syntax alone cannot say where a call goes. `use std::fs; fs::read(path)`
//! and `std::fs::read(path)` are the same call written two ways, and only name
//! resolution knows it. This binary replaces `rustc` for one compilation,
//! reads the resolved mid-level representation, and writes what it learned as
//! span-anchored observations.
//!
//! It observes; it does not decide. Whether a resolved call matters is a
//! question for a consumer, and the schema it emits says nothing about Rust.

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;
extern crate rustc_session;
extern crate rustc_span;

use std::ops::ControlFlow;

use entl_semantics::{
    CallEdge, Definition, Dispatch, EntityId, EntityKind, Gap, Provenance, SemanticObservations,
    Span, Visibility,
};
use rustc_public::mir::{MirVisitor, Terminator, TerminatorKind};
use rustc_public::ty::{RigidTy, TyKind};
use rustc_public::{CrateDef, CrateItem};

/// Where to write the observations, taken from the environment because the
/// argument list belongs to the compiler.
const OUTPUT_VARIABLE: &str = "ENTL_RUST_MIR_OUTPUT";

fn main() -> std::process::ExitCode {
    let arguments = std::env::args().collect::<Vec<_>>();
    match rustc_public::run!(&arguments, observe) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        // `rustc --print`, `-vV` and friends compile nothing, and a build tool
        // asks those before it asks for a compilation. Reporting failure there
        // makes this unusable as a drop-in `RUSTC`, which is the only way a
        // normal build produces observations at all.
        Err(rustc_public::CompilerError::Skipped) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("entl-rust-mir: {error:?}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn observe() -> ControlFlow<()> {
    let mut observations = SemanticObservations::new(Provenance {
        provider: "rust.mir".to_owned(),
        provider_version: env!("CARGO_PKG_VERSION").to_owned(),
        toolchain: toolchain(),
        unit: rustc_public::local_crate().name,
    });
    // This provider resolves calls and enumerates definitions. It does not yet
    // record types, references, or implementations, and says so rather than
    // leaving a consumer to guess from their absence.
    observations.coverage.definitions = true;
    observations.coverage.call_edges = true;

    for item in rustc_public::all_local_items() {
        let id = entity_id(&item);
        let span = match convert_span(&item.span()) {
            Ok(span) => Some(span),
            Err(reason) => {
                observations.gaps.push(Gap {
                    span: None,
                    message: format!("{} has no usable span: {reason}", item.name()),
                });
                None
            }
        };
        observations.definitions.push(Definition {
            id: id.clone(),
            kind: entity_kind(&item),
            name: item.name(),
            container: None,
            visibility: Visibility::Unknown,
            span: span.clone(),
        });

        let instance = match rustc_public::mir::mono::Instance::try_from(item) {
            Ok(instance) => instance,
            Err(error) => {
                // a generic or otherwise non-monomorphic item has no instance
                // to read; that is a gap, not an absence of calls
                observations.gaps.push(Gap {
                    span: span.clone(),
                    message: format!("{} has no instance to read: {error:?}", item.name()),
                });
                continue;
            }
        };
        let Some(body) = instance.body() else {
            // a body can be absent for an intrinsic or a foreign item; that is
            // a gap in what was observed, not an absence of calls
            observations.gaps.push(Gap {
                span,
                message: format!("{} has no body to read", item.name()),
            });
            continue;
        };

        let mut calls = CallCollector {
            from: id,
            edges: Vec::new(),
            gaps: Vec::new(),
        };
        calls.visit_body(&body);
        observations.call_edges.extend(calls.edges);
        observations.gaps.extend(calls.gaps);
    }

    observations.canonicalize();
    if let Err(reason) = write(&observations) {
        eprintln!("entl-rust-mir: {reason}");
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}

/// Collects one edge per call terminator in a body.
struct CallCollector {
    from: EntityId,
    edges: Vec<CallEdge>,
    /// Calls seen but not recordable, so a short edge list is never silent.
    gaps: Vec<Gap>,
}

impl MirVisitor for CallCollector {
    fn visit_terminator(
        &mut self,
        terminator: &Terminator,
        location: rustc_public::mir::visit::Location,
    ) {
        if let TerminatorKind::Call { func, .. } = &terminator.kind {
            let (to, dispatch) = match func.ty(&[]).map(|ty| ty.kind()) {
                // a call to a known function: the type itself names the callee
                Ok(TyKind::RigidTy(RigidTy::FnDef(def, arguments))) => (
                    vec![EntityId::new(destination(def, &arguments))],
                    Dispatch::Static,
                ),
                // a function pointer or closure the compiler did not settle
                _ => (Vec::new(), Dispatch::Unknown),
            };
            match convert_span(&terminator.source_info.span) {
                Ok(span) => self.edges.push(CallEdge {
                    span,
                    from: self.from.clone(),
                    to,
                    dispatch,
                }),
                Err(reason) => self.gaps.push(Gap {
                    span: None,
                    message: format!("a call in {:?} has no usable span: {reason}", self.from),
                }),
            }
        }
        self.super_terminator(terminator, location);
    }
}

/// The definition a call actually enters, not the one it was written against.
///
/// A trait method names the trait until it is monomorphized: cloning an `Arc`
/// and cloning a `String` are both `std::clone::Clone::clone`, and the
/// difference between them — one bumps a count, the other copies a buffer —
/// lives in the generic arguments. Resolving the instance puts the receiver
/// back into the name, which is the whole reason a consumer asks a compiler
/// rather than the syntax.
///
/// Resolution fails for a call the compiler itself could not settle, and the
/// unresolved name is still better than no edge at all.
fn destination(def: rustc_public::ty::FnDef, arguments: &rustc_public::ty::GenericArgs) -> String {
    rustc_public::mir::mono::Instance::resolve(def, arguments)
        .map(|instance| instance.name())
        .unwrap_or_else(|_| def.0.name())
}

fn entity_id(item: &CrateItem) -> EntityId {
    EntityId::new(item.name())
}

fn entity_kind(item: &CrateItem) -> EntityKind {
    match item.kind() {
        rustc_public::ItemKind::Fn => EntityKind::Function,
        rustc_public::ItemKind::Static | rustc_public::ItemKind::Const => EntityKind::Constant,
        _ => EntityKind::Other,
    }
}

/// Convert a compiler span, or say which coordinate would not fit.
///
/// A dropped span costs a call edge, and a missing edge silently weakens every
/// consumer that reasons over the call graph, so the failure is named rather
/// than folded into an absence.
fn convert_span(span: &rustc_public::ty::Span) -> Result<Span, String> {
    let lines = span.get_lines();
    let coordinate = |name: &str, value: usize| {
        u32::try_from(value).map_err(|_| format!("{name} {value} does not fit a span coordinate"))
    };
    Ok(Span {
        path: std::path::PathBuf::from(span.get_filename()),
        start_line: coordinate("start line", lines.start_line)?,
        start_column: coordinate("start column", lines.start_col)?,
        end_line: coordinate("end line", lines.end_line)?,
        end_column: coordinate("end column", lines.end_col)?,
    })
}

fn toolchain() -> String {
    env!("ENTL_RUST_MIR_TOOLCHAIN").to_owned()
}

/// Write the observations, or say why they are not there.
///
/// Every failure here produces a run that compiled cleanly and recorded
/// nothing, which a consumer cannot distinguish from a crate that genuinely
/// had nothing to say. None of them may be swallowed.
fn write(observations: &SemanticObservations) -> Result<(), String> {
    let directory = std::env::var(OUTPUT_VARIABLE).map_err(|error| {
        format!("reading {OUTPUT_VARIABLE}, which says where observations go: {error}")
    })?;
    let encoded = serde_json::to_vec_pretty(observations)
        .map_err(|error| format!("encoding observations: {error}"))?;
    // one file per compiled crate, so a workspace build does not race
    let path = std::path::PathBuf::from(directory).join(format!(
        "{}.json",
        observations.provenance.unit.replace(['/', ' '], "-")
    ));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    }
    std::fs::write(&path, encoded).map_err(|error| format!("writing {}: {error}", path.display()))
}
