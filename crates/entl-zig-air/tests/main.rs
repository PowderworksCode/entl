// Tests for `src/main.rs`: the binary's front door.
//
// A binary is only testable by running it, so this is a smoke test: it checks
// the thing builds and refuses a run it cannot complete, rather than exiting
// zero having written nothing.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

#[test]
fn the_binary_refuses_to_run_without_an_output_directory() {
    let output = Command::new(env!("CARGO_BIN_EXE_entl-zig-air"))
        .output()
        .expect("the binary runs");
    assert!(
        !output.status.success(),
        "a run with no output directory should not report success"
    );
}
