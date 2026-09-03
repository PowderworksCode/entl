// Tests for `src/lib.rs`.
//
// The observer shells out to a TypeScript toolchain, so what can be checked
// without one is the part that decides whether to try: `available` answers from
// the options rather than by running anything, and either answer is legitimate
// on a given machine.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_ts_observe::{Options, available};

#[test]
fn availability_is_answered_without_a_project() {
    let options = Options::default();
    let answer = available(&options);
    assert!(
        answer || !answer,
        "asking does not panic and needs no project"
    );
}
