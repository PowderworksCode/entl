// Tests for `src/main.rs`: the binary's front door.
//
// This binary is a rustc driver — it runs inside the compiler, so it cannot be
// exercised from a test without a toolchain. What can be checked is that it
// builds and refuses a run it cannot complete rather than exiting zero.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

#[test]
fn the_binary_refuses_a_run_with_no_crate_to_observe() {
    let output = Command::new(env!("CARGO_BIN_EXE_entl-rust-mir"))
        .output()
        .expect("the binary runs");
    assert!(
        !output.status.success(),
        "a run with nothing to observe should not report success"
    );
}
