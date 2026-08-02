//! Observes resolved TypeScript semantics by running the compiler's checker.
//!
//! The observation itself is a Node program, because the TypeScript checker is
//! written in TypeScript and there is no other way to ask it anything. This
//! crate runs that program over a project and reads back what it saw.
//!
//! Unlike the Rust provider, nothing here needs a pinned toolchain or a
//! compiler's private crates: the checker is an ordinary dependency of the
//! project being observed. That is why this crate is an ordinary workspace
//! member and is covered by the same `cargo test` and `cargo clippy` as
//! everything else, rather than sitting outside every automated check.
//!
//! It observes; it does not decide. Whether a resolved type matters is a
//! question for a consumer, and the schema it produces says nothing about
//! TypeScript.

use std::path::{Path, PathBuf};
use std::process::Command;

use entl_semantics::SemanticObservations;

/// Where the observing program lives, relative to this crate.
const OBSERVER: &str = "../../providers/typescript/observe.mjs";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("running {} to observe {}: {source}", program.display(), project.display())]
    Spawn {
        program: PathBuf,
        project: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("observing {}: {message}", project.display())]
    Observer { project: PathBuf, message: String },
    #[error("reading the observations of {}: {source}", project.display())]
    Read {
        project: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("decoding the observations of {}: {source}", project.display())]
    Decode {
        project: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("the observing program is not at {}", path.display())]
    Missing { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, Error>;

/// How to run the observer.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The TypeScript compiler to run.
    ///
    /// Defaults to the one the project itself builds with, because the
    /// observations describe that build and not some other one.
    pub typescript: Option<PathBuf>,
    /// The Node executable, when it is not simply `node`.
    pub node: Option<PathBuf>,
}

/// The program that does the observing.
fn observer() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(OBSERVER)
}

/// Whether this machine can observe TypeScript at all.
///
/// A consumer needs to tell "there was nothing to see" from "nothing looked",
/// and a missing Node is the second. Reporting it as an empty result would be
/// a confident claim about a project nobody read.
#[must_use]
pub fn available(options: &Options) -> bool {
    if !observer().is_file() {
        return false;
    }
    let node = options
        .node
        .clone()
        .unwrap_or_else(|| PathBuf::from("node"));
    Command::new(node)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Observe one TypeScript project.
pub fn observe(project: impl AsRef<Path>, options: &Options) -> Result<SemanticObservations> {
    let project = project.as_ref().to_path_buf();
    let program = observer();
    if !program.is_file() {
        return Err(Error::Missing { path: program });
    }
    let output_directory = std::env::temp_dir().join("entl-ts-observe");
    std::fs::create_dir_all(&output_directory).map_err(|source| Error::Read {
        project: project.clone(),
        source,
    })?;
    let output_path = output_directory.join(format!(
        "{}.json",
        project.file_name().map_or_else(
            || "project".to_owned(),
            |name| name.to_string_lossy().replace(['/', ' '], "-")
        )
    ));

    let node = options
        .node
        .clone()
        .unwrap_or_else(|| PathBuf::from("node"));
    let mut command = Command::new(node);
    command
        .arg(&program)
        .arg("--project")
        .arg(&project)
        .arg("--out")
        .arg(&output_path);
    if let Some(typescript) = &options.typescript {
        command.arg("--typescript").arg(typescript);
    }
    let output = command.output().map_err(|source| Error::Spawn {
        program: program.clone(),
        project: project.clone(),
        source,
    })?;
    if !output.status.success() {
        // a failure that produced no observations must be said out loud; an
        // empty result would read downstream as a project with nothing in it
        return Err(Error::Observer {
            project,
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    let encoded = std::fs::read(&output_path).map_err(|source| Error::Read {
        project: project.clone(),
        source,
    })?;
    let mut observed: SemanticObservations =
        serde_json::from_slice(&encoded).map_err(|source| Error::Decode {
            project: project.clone(),
            source,
        })?;
    observed.canonicalize();
    Ok(observed)
}
