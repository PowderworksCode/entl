//! Checks the generated verbosity table against the profiles it describes.
//!
//! The numbers themselves come from `tools/rosetta-verbosity` and change when
//! the corpus does. What must hold regardless is that the table names only real
//! languages, stays binary-searchable, and agrees with itself.

use std::collections::BTreeSet;

use entl_codebase::{
    VERBOSITY_BASELINE, language_profile, language_profiles, verbosity, verbosity_ratio,
    verbosity_ratios,
};

#[test]
fn every_measured_language_is_a_registered_profile() {
    for (left, right, _) in verbosity_ratios() {
        for language in [left, right] {
            assert!(
                language_profile(language).is_some(),
                "verbosity table names unregistered language {language:?}"
            );
            assert!(
                verbosity(language).is_some(),
                "language {language:?} has a ratio but no index"
            );
        }
    }
}

#[test]
fn pairs_are_sorted_deduplicated_and_ordered_left_to_right() {
    let pairs = verbosity_ratios()
        .map(|(left, right, _)| (left, right))
        .collect::<Vec<_>>();
    assert!(!pairs.is_empty());
    assert!(pairs.iter().all(|(left, right)| left < right));
    assert!(pairs.windows(2).all(|window| window[0] < window[1]));
    assert_eq!(pairs.iter().collect::<BTreeSet<_>>().len(), pairs.len());
}

#[test]
fn the_baseline_is_a_measured_language_at_one() {
    let baseline = verbosity(VERBOSITY_BASELINE).expect("baseline is measured");
    assert_eq!(baseline.bytes, 1.0);
    assert_eq!(baseline.lines, 1.0);
}

#[test]
fn a_ratio_reads_the_same_from_either_side() {
    let (left, right, forward) = verbosity_ratios().next().expect("at least one pair");
    let reverse = verbosity_ratio(right, left).expect("reverse direction is available");
    assert_eq!(reverse, forward.inverted());
    assert_eq!(verbosity_ratio(left, right), Some(forward));
    assert_eq!(verbosity_ratio(left, left), None);
    assert_eq!(verbosity_ratio(left, "not-a-language"), None);
}

#[test]
fn measured_pairs_stay_within_their_reported_deviation() {
    for (left, right, ratio) in verbosity_ratios() {
        let (left_index, right_index) = (
            verbosity(left).expect("index"),
            verbosity(right).expect("index"),
        );
        let fitted = left_index.bytes / right_index.bytes;
        let gap = (ratio.bytes / fitted - 1.0).abs();
        let allowed = left_index.deviation.max(right_index.deviation);
        assert!(
            gap <= allowed + 1e-3,
            "{left}/{right} is {gap} off the fitted ratio but reports at most {allowed}"
        );
    }
}

#[test]
fn languages_absent_from_the_corpus_carry_no_measurement() {
    for id in ["css", "html", "json", "yaml", "markdown"] {
        let profile = language_profile(id).expect("profile is registered");
        assert!(
            profile.verbosity().is_none(),
            "{id} has no algorithmic presence in the corpus but reports a verbosity"
        );
    }
}

#[test]
fn the_profile_method_and_the_free_function_agree() {
    for profile in language_profiles() {
        assert_eq!(profile.verbosity(), verbosity(profile.id));
    }
}
