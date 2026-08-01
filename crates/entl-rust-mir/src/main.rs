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
        Err(_) => std::process::ExitCode::FAILURE,
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
        observations.definitions.push(Definition {
            id: id.clone(),
            kind: entity_kind(&item),
            name: item.name(),
            container: None,
            visibility: Visibility::Unknown,
            span: convert_span(&item.span()),
        });

        let instance = match rustc_public::mir::mono::Instance::try_from(item) {
            Ok(instance) => instance,
            Err(_) => continue,
        };
        let Some(body) = instance.body() else {
            // a body can be absent for an intrinsic or a foreign item; that is
            // a gap in what was observed, not an absence of calls
            observations.gaps.push(Gap {
                span: convert_span(&item.span()),
                message: format!("{} has no body to read", item.name()),
            });
            continue;
        };

        let mut calls = CallCollector {
            from: id,
            edges: Vec::new(),
        };
        calls.visit_body(&body);
        observations.call_edges.extend(calls.edges);
    }

    observations.canonicalize();
    write(&observations);
    ControlFlow::Continue(())
}

/// Collects one edge per call terminator in a body.
struct CallCollector {
    from: EntityId,
    edges: Vec<CallEdge>,
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
                Ok(TyKind::RigidTy(RigidTy::FnDef(def, _))) => {
                    (vec![EntityId::new(def.0.name())], Dispatch::Static)
                }
                // a function pointer or closure the compiler did not settle
                _ => (Vec::new(), Dispatch::Unknown),
            };
            if let Some(span) = convert_span(&terminator.source_info.span) {
                self.edges.push(CallEdge {
                    span,
                    from: self.from.clone(),
                    to,
                    dispatch,
                });
            }
        }
        self.super_terminator(terminator, location);
    }
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

fn convert_span(span: &rustc_public::ty::Span) -> Option<Span> {
    let lines = span.get_lines();
    Some(Span {
        path: std::path::PathBuf::from(span.get_filename()),
        start_line: u32::try_from(lines.start_line).ok()?,
        start_column: u32::try_from(lines.start_col).ok()?,
        end_line: u32::try_from(lines.end_line).ok()?,
        end_column: u32::try_from(lines.end_col).ok()?,
    })
}

fn toolchain() -> String {
    env!("ENTL_RUST_MIR_TOOLCHAIN").to_owned()
}

fn write(observations: &SemanticObservations) {
    let Ok(path) = std::env::var(OUTPUT_VARIABLE) else {
        return;
    };
    let Ok(encoded) = serde_json::to_vec_pretty(observations) else {
        return;
    };
    // one file per compiled crate, so a workspace build does not race
    let path = std::path::PathBuf::from(path).join(format!(
        "{}.json",
        observations.provenance.unit.replace(['/', ' '], "-")
    ));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, encoded);
}
