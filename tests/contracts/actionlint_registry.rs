//! Cases for the comparison between the actionlint registry and the labels in use.
//!
//! The contract in `placement` reads `.github/actionlint.yaml` and the
//! workflows. These cases hold the comparison itself, with no file read, to an
//! exact label, a trailing `*` glob and both ways a registry can be wrong.

use std::collections::BTreeSet;

use rstest::rstest;

use crate::workflow_config::registry_mismatches;

/// Builds an owned list of labels from string literals.
fn owned(labels: &[&str]) -> Vec<String> {
    labels.iter().map(|label| (*label).to_owned()).collect()
}

/// Compares `declared` with `in_use`, returning the two mismatch lists as owned values.
fn mismatches(declared: &[&str], in_use: &[&str]) -> (Vec<String>, Vec<String>) {
    let registry = owned(declared);
    let labels: BTreeSet<String> = owned(in_use).into_iter().collect();
    let (unregistered, unused) = registry_mismatches(&registry, &labels);
    (
        unregistered.into_iter().cloned().collect(),
        unused.into_iter().cloned().collect(),
    )
}

/// A registry that names exactly what is used, by label or by glob, has no mismatch.
#[rstest]
#[case::exact(&["ubicloud-standard-4"], &["ubicloud-standard-4"])]
#[case::glob_over_one_shape(&["ubicloud-*"], &["ubicloud-standard-4"])]
#[case::glob_over_two_shapes(&["ubicloud-*"], &["ubicloud-standard-2", "ubicloud-standard-4"])]
#[case::exact_and_glob(&["ubicloud-*", "self-hosted-x"], &["ubicloud-standard-4", "self-hosted-x"])]
#[case::nothing_in_use_nothing_declared(&[], &[])]
fn a_matching_registry_has_no_mismatch(#[case] declared: &[&str], #[case] in_use: &[&str]) {
    assert_eq!(mismatches(declared, in_use), (Vec::new(), Vec::new()));
}

/// A label no entry covers is reported, however close an entry comes.
#[rstest]
#[case::empty_registry(&[], &["ubicloud-standard-4"], &["ubicloud-standard-4"])]
#[case::a_shorter_exact_entry(&["ubicloud-standard"], &["ubicloud-standard-4"], &["ubicloud-standard-4"])]
#[case::a_glob_for_another_family(&["ubuntu-*"], &["ubicloud-standard-4"], &["ubicloud-standard-4"])]
#[case::a_glob_needs_its_whole_prefix(&["ubicloud-standard-*"], &["ubicloud-arm-4"], &["ubicloud-arm-4"])]
fn an_uncovered_label_is_reported_as_unregistered(
    #[case] declared: &[&str],
    #[case] in_use: &[&str],
    #[case] expected: &[&str],
) {
    let (unregistered, _) = mismatches(declared, in_use);
    assert_eq!(unregistered, owned(expected));
}

/// An entry that covers no label in use is reported, so a retired runner cannot linger.
#[rstest]
#[case::exact_entry_not_used(&["ubicloud-standard-4", "ubicloud-standard-8"], &["ubicloud-standard-4"], &["ubicloud-standard-8"])]
#[case::glob_matching_nothing(&["ubicloud-*"], &["ubuntu-latest"], &["ubicloud-*"])]
#[case::registry_with_no_workflow_use(&["ubicloud-*"], &[], &["ubicloud-*"])]
fn an_entry_that_covers_nothing_is_reported_as_unused(
    #[case] declared: &[&str],
    #[case] in_use: &[&str],
    #[case] expected: &[&str],
) {
    let (_, unused) = mismatches(declared, in_use);
    assert_eq!(unused, owned(expected));
}

/// An exact entry is compared whole, never as a substring of a longer label.
#[rstest]
fn an_exact_entry_does_not_match_a_longer_label() {
    let (unregistered, unused) = mismatches(&["standard-8"], &["ubicloud-standard-8"]);
    assert_eq!(unregistered, owned(&["ubicloud-standard-8"]));
    assert_eq!(unused, owned(&["standard-8"]));
}
