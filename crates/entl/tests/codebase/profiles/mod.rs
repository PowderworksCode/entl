// Tests for `src/codebase/profiles/mod.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl::codebase::{ecosystem_profiles, language_profiles};

#[test]
fn built_in_profiles_are_complete_and_deterministic() {
    let languages = language_profiles()
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let ecosystems = ecosystem_profiles()
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();

    assert!(languages.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(languages.contains(&"rust"));
    assert!(languages.contains(&"javascript"));
    assert!(languages.contains(&"typescript"));
    assert_eq!(ecosystems, ["bun", "cargo", "npm", "pnpm", "yarn"]);
}
