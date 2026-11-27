//! Cases for the comparison between the actionlint registry and the labels in use.
//!
//! The contract in `placement` reads `.github/actionlint.yaml` and the
//! workflows. These cases hold the comparison itself, with no file read, to an
//! exact label, a trailing `*` glob and both ways a registry can be wrong. Every
//! case states both mismatch lists, so a comparison that fills one from the
//! other fails here.

use std::collections::BTreeSet;

use rstest::rstest;

use crate::workflow_config::{registry_entry_covers, registry_mismatches};

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
    assert_eq!(
        mismatches(declared, in_use),
        (Vec::new(), Vec::new()),
        "registry {declared:?} must name exactly the labels {in_use:?}, with no mismatch"
    );
}

/// A label no entry covers is unregistered, and an entry that covers no label is unused.
///
/// Each case lists the expected unregistered labels, then the expected unused entries.
#[rstest]
#[case::empty_registry(&[], &["ubicloud-standard-4"], &["ubicloud-standard-4"], &[])]
#[case::a_shorter_exact_entry(
    &["ubicloud-standard"], &["ubicloud-standard-4"], &["ubicloud-standard-4"], &["ubicloud-standard"]
)]
#[case::a_glob_for_another_family(
    &["ubuntu-*"], &["ubicloud-standard-4"], &["ubicloud-standard-4"], &["ubuntu-*"]
)]
#[case::a_glob_needs_its_whole_prefix(
    &["ubicloud-standard-*"], &["ubicloud-arm-4"], &["ubicloud-arm-4"], &["ubicloud-standard-*"]
)]
#[case::a_glob_needs_the_hyphen(&["ubicloud-*"], &["ubicloud"], &["ubicloud"], &["ubicloud-*"])]
#[case::exact_entry_not_used(
    &["ubicloud-standard-4", "ubicloud-standard-8"], &["ubicloud-standard-4"], &[], &["ubicloud-standard-8"]
)]
#[case::glob_matching_nothing(&["ubicloud-*"], &["ubuntu-latest"], &["ubuntu-latest"], &["ubicloud-*"])]
#[case::registry_with_no_workflow_use(&["ubicloud-*"], &[], &[], &["ubicloud-*"])]
#[case::an_exact_entry_is_not_a_substring(
    &["standard-8"], &["ubicloud-standard-8"], &["ubicloud-standard-8"], &["standard-8"]
)]
fn a_mismatched_registry_reports_both_lists(
    #[case] declared: &[&str],
    #[case] in_use: &[&str],
    #[case] unregistered: &[&str],
    #[case] unused: &[&str],
) {
    assert_eq!(
        mismatches(declared, in_use),
        (owned(unregistered), owned(unused)),
        "registry {declared:?} against labels {in_use:?} must report unregistered \
         {unregistered:?} and unused {unused:?}"
    );
}

/// An entry covers a label exactly, or as a trailing `*` glob over its prefix.
#[rstest]
#[case::exact_match("ubicloud-standard-4", "ubicloud-standard-4", true)]
#[case::exact_differs("ubicloud-standard-4", "ubicloud-standard-2", false)]
#[case::glob_prefix("ubicloud-*", "ubicloud-standard-4", true)]
#[case::glob_needs_the_hyphen("ubicloud-*", "ubicloud", false)]
#[case::glob_other_family("ubicloud-*", "ubuntu-latest", false)]
#[case::bare_star_covers_everything("*", "anything", true)]
#[case::exact_is_not_a_substring("standard-8", "ubicloud-standard-8", false)]
fn an_entry_covers_a_label_exactly_or_by_trailing_glob(
    #[case] entry: &str,
    #[case] label: &str,
    #[case] expected: bool,
) {
    assert_eq!(
        registry_entry_covers(entry, label),
        expected,
        "entry {entry:?} against label {label:?} must {}cover it",
        if expected { "" } else { "not " }
    );
}
