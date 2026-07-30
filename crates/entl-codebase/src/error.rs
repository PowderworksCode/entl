use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not inspect codebase root {path}: {source}")]
    Root {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid inventory ignore pattern {pattern:?}: {message}")]
    IgnorePattern { pattern: String, message: String },

    #[error("codebase-relative path is unsafe: {0}")]
    UnsafePath(PathBuf),

    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path} is not valid UTF-8")]
    NonUtf8 { path: PathBuf },
}

impl Error {
    pub(crate) fn root(path: &Path, source: std::io::Error) -> Self {
        Self::Root {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
