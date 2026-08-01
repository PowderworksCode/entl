//! Links the binary against the toolchain it will run inside.
//!
//! A rustc driver loads the compiler's own dynamic libraries at run time, and
//! nothing on the default search path provides them. Recording the active
//! sysroot as an rpath is what lets the binary start at all.

use std::process::Command;

fn main() {
    let output = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()))
        .args(["--print", "sysroot"])
        .output()
        .expect("asking the active toolchain for its sysroot");
    let sysroot = String::from_utf8(output.stdout).expect("sysroot path is text");
    let library_path = format!("{}/lib", sysroot.trim());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{library_path}");

    // The driver is the compiler it was built against, so the toolchain is
    // known here and not at run time. Recording it lets a consumer invalidate
    // observations when the compiler changes and not only when source does.
    let version = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()))
        .arg("--version")
        .output()
        .expect("asking the active toolchain for its version");
    let version = String::from_utf8(version.stdout).expect("version is text");
    println!("cargo:rustc-env=ENTL_RUST_MIR_TOOLCHAIN={}", version.trim());
    println!("cargo:rerun-if-changed=build.rs");
}
