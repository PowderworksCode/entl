// Tests for `src/github/pin.rs`.
//
// The policy is what decides whether `uses: owner/action@ref` is pinned. A
// commit sha is 40 hex characters; the channels are the moving refs a repository
// may deliberately follow instead.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::github::ACTION_PINS;

#[test]
fn a_pin_is_a_full_commit_sha() {
    assert_eq!(ACTION_PINS.commit_sha_length, 40);
}

#[test]
fn the_allowed_channels_are_named_and_non_empty() {
    assert!(!ACTION_PINS.allowed_channels.is_empty());
    for channel in ACTION_PINS.allowed_channels {
        assert!(!channel.is_empty());
    }
}
