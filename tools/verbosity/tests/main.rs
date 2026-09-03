// Tests for `src/main.rs`: the binary's front door.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::process::Command;

#[test]
fn the_binary_refuses_a_run_with_no_corpus() {
    let output = Command::new(env!("CARGO_BIN_EXE_verbosity"))
        .output()
        .expect("the binary runs");
    assert!(
        !output.status.success(),
        "a run with no corpus should not report success"
    );
}
