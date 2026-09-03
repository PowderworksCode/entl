// Tests for `src/codebase/compiler.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::Path;

use entl::codebase::parse_rustc;

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
