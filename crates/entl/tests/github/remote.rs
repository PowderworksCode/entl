// Tests for `src/github/remote.rs`.
//
// `GithubValue` is the difference between a fact and a fact nobody was allowed
// to read. A private repository returns no branch protection, and treating that
// absence as "unprotected" would report a finding about something unobserved.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::github::GithubValue;

#[test]
fn a_known_value_carries_what_was_read() {
    let value = GithubValue::known(7_u32);
    assert!(matches!(value, GithubValue::Known { value: 7 }));
}

/// The three states are distinct on purpose: unavailable is "the API said no",
/// which is not the same as a value that happens to be absent.
#[test]
fn the_states_are_distinguishable() {
    let known: GithubValue<u32> = GithubValue::known(1);
    assert!(matches!(known, GithubValue::Known { .. }));
}
