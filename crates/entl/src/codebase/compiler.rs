use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::codebase::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerObservation {
    pub name: String,
    pub version: String,
    pub commit: Option<String>,
    pub host: String,
    pub sysroot: PathBuf,
    pub standard_library_source: Option<PathBuf>,
    pub cfg: BTreeSet<String>,
    pub target_features: BTreeSet<String>,
}

pub fn observe_rust_compiler(root: impl AsRef<Path>) -> Result<CompilerObservation> {
    let root = root.as_ref();
    let verbose = run_rustc(root, &["--version", "--verbose"])?;
    let cfg = run_rustc(root, &["--print", "cfg"])?;
    let sysroot = run_rustc(root, &["--print", "sysroot"])?;
    parse_rustc(&verbose, &cfg, sysroot.trim())
}

fn run_rustc(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("rustc")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|source| Error::Command {
            program: "rustc".to_owned(),
            source,
        })?;
    if !output.status.success() {
        return Err(Error::CommandFailed {
            program: "rustc".to_owned(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_rustc(verbose: &str, cfg: &str, sysroot: &str) -> Result<CompilerObservation> {
    let field = |name: &str| {
        verbose
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let version = field("release:").or_else(|| {
        verbose
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .map(str::to_owned)
    });
    let Some(version) = version else {
        return Err(Error::CommandOutput {
            program: "rustc".to_owned(),
            message: "verbose version output has no release".to_owned(),
        });
    };
    let Some(host) = field("host:") else {
        return Err(Error::CommandOutput {
            program: "rustc".to_owned(),
            message: "verbose version output has no host".to_owned(),
        });
    };
    if sysroot.is_empty() {
        return Err(Error::CommandOutput {
            program: "rustc".to_owned(),
            message: "sysroot output is empty".to_owned(),
        });
    }
    let sysroot = PathBuf::from(sysroot);
    let source = sysroot.join("lib/rustlib/src/rust/library");
    let standard_library_source = source.is_dir().then_some(source);
    let cfg = cfg
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let target_features = cfg
        .iter()
        .filter_map(|value| {
            value
                .strip_prefix("target_feature=\"")
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect();
    Ok(CompilerObservation {
        name: "rustc".to_owned(),
        version,
        commit: field("commit-hash:"),
        host,
        sysroot,
        standard_library_source,
        cfg,
        target_features,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::parse_rustc;

    #[test]
    fn parses_verbose_version_and_target_configuration() {
        let compiler = parse_rustc(
            "rustc 1.93.1 (01f4d6f7f 2026-02-11)\nbinary: rustc\ncommit-hash: 01f4d6f7f\nhost: aarch64-apple-darwin\nrelease: 1.93.1\n",
            "target_arch=\"aarch64\"\ntarget_feature=\"aes\"\ntarget_feature=\"neon\"\n",
            "/toolchains/stable",
        )
        .unwrap();
        assert_eq!(compiler.version, "1.93.1");
        assert_eq!(compiler.commit.as_deref(), Some("01f4d6f7f"));
        assert_eq!(compiler.host, "aarch64-apple-darwin");
        assert_eq!(compiler.sysroot, Path::new("/toolchains/stable"));
        assert_eq!(compiler.standard_library_source, None);
        assert_eq!(
            compiler.target_features,
            ["aes", "neon"].map(str::to_owned).into()
        );
    }
}
