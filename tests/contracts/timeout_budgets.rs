//! The readings the timeout ordering rests on, driven directly.
//!
//! Every ceiling in this estate sits well above its requirement and both
//! lanes carry the same watchdog, so the assertions over the workflows
//! cannot tell a correct reading from several plausible wrong ones. These
//! drive the readings with values chosen to separate them, and with
//! `proptest` where the domain is too large to enumerate, following the
//! split [ADR 003](../../docs/adr-003-bounded-rstest-over-property-testing.md)
//! records.
//!
//! Separated from `timeouts`, whose subject is the workflows as they
//! stand rather than the arithmetic applied to them.

use rstest::rstest;

use crate::shared_action;

/// Everything in a coverage job that is not the `cargo` invocation the
/// watchdog bounds: checkout, toolchain setup, cache restore, and
/// whatever follows the coverage step. The job timer covers it; the
/// watchdog does not.
///
/// Measured from the worst of several runs rather than one. Across
/// fifteen successful `ci.yml` runs the widest gap between the coverage
/// step and its job was 859 s on run 33830336409; across twenty of
/// `coverage-main.yml` it was 286 s on run 31892219565. Fifteen minutes
/// covers the worse of those, and none of those runs was genuinely cold.
pub const OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS: u64 = 15 * 60;

/// The name of the coverage action, without its owner or pin.
pub const COVERAGE_ACTION: &str = "generate-coverage";

/// Returns whether a step invokes the coverage action.
///
/// The `uses` value is the coordinate, an `@`, and a pin. Matching the
/// coordinate as a prefix of the whole string would also match a
/// sibling action whose name merely begins with this one's, such as
/// `generate-coverage-variant`, and that action has no watchdog of its
/// own for the assertions below to be about.
///
/// # Examples
///
/// ```ignore
/// let coordinate = "leynos/shared-actions/.github/actions/generate-coverage";
/// assert!(invokes_coverage(&format!("{coordinate}@abc123"), coordinate));
/// assert!(!invokes_coverage(&format!("{coordinate}-variant@abc123"), coordinate));
/// ```
pub fn invokes_coverage(uses: &str, coordinate: &str) -> bool {
    uses.split('@').next().unwrap_or(uses) == coordinate
}

/// Returns whether a ceiling contains a watchdog and the work around it.
///
/// Extracted so the decision is one named thing and the three messages
/// below are only messages. `None` is a job that declares no ceiling at
/// all, which is not a smaller number but a different failure: GitHub's
/// six-hour default applies and nothing in the workflow says so.
///
/// # Examples
///
/// ```ignore
/// // 3,600 s of watchdog and 900 s of work around it need 4,500 s.
/// assert!(ceiling_covers_watchdog_budget(Some(5_400), 3_600));
/// assert!(!ceiling_covers_watchdog_budget(Some(4_499), 3_600));
/// assert!(!ceiling_covers_watchdog_budget(None, 3_600));
/// ```
pub fn ceiling_covers_watchdog_budget(ceiling_seconds: Option<u64>, watchdog: u64) -> bool {
    ceiling_seconds.is_some_and(|seconds| seconds >= required_ceiling(watchdog))
}

/// Returns the smallest acceptable ceiling for one watchdog, in seconds.
///
/// Saturating rather than wrapping. The watchdog is parsed from a
/// workflow, so a value near `u64::MAX` is reachable by editing a file;
/// a wrapping add would turn it into a small requirement that every
/// ceiling satisfies, which is the opposite of what a preposterous
/// watchdog should produce.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(required_ceiling(3_600), 4_500);
/// assert_eq!(required_ceiling(u64::MAX), u64::MAX);
/// ```
pub const fn required_ceiling(watchdog: u64) -> u64 {
    watchdog.saturating_add(OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS)
}

/// Returns one job's `timeout-minutes` in seconds.
///
/// Saturating for the same reason the requirement above is: the value
/// is parsed from a workflow, so a ceiling beyond `u64::MAX / 60` is
/// reachable by editing a file, and multiplying it would wrap to a
/// small number that then fails the ordering for the wrong reason. A
/// saturated ceiling passes, which is right: a ceiling that large is
/// preposterous but it is not an inversion, and the contract reports
/// inversions.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(ceiling_seconds(90), 5_400);
/// assert_eq!(ceiling_seconds(u64::MAX), u64::MAX);
/// ```
pub const fn ceiling_seconds(minutes: u64) -> u64 {
    minutes.saturating_mul(60)
}

#[rstest]
#[case::the_action_itself("leynos/shared-actions/.github/actions/generate-coverage@abc", true)]
#[case::an_unpinned_reference("leynos/shared-actions/.github/actions/generate-coverage", true)]
#[case::a_sibling_with_a_longer_name(
    "leynos/shared-actions/.github/actions/generate-coverage-variant@abc",
    false
)]
#[case::a_different_action("leynos/shared-actions/.github/actions/install-nixie@abc", false)]
#[case::a_different_owner("someone/shared-actions/.github/actions/generate-coverage@abc", false)]
fn the_coverage_coordinate_matches_the_whole_action_path(
    #[case] uses: &str,
    #[case] expected: bool,
) {
    // Every workflow in this tree pins the action itself, so the
    // assertions above are satisfied by a prefix match that would also
    // claim a sibling action beginning with the same name. That sibling
    // has no watchdog of its own, so the claim would be a failing
    // assertion about a job that never runs coverage.
    let coordinate = shared_action(COVERAGE_ACTION);
    assert_eq!(
        invokes_coverage(uses, &coordinate),
        expected,
        "{uses:?} must {} the coverage action",
        if expected { "match" } else { "not match" }
    );
}

#[rstest]
#[case::comfortably_above(Some(5_400), 3_600, true)]
#[case::exactly_the_requirement(Some(4_500), 3_600, true)]
#[case::one_second_short(Some(4_499), 3_600, false)]
#[case::no_ceiling_at_all(None, 3_600, false)]
fn the_ceiling_predicate_decides_the_three_cases(
    #[case] ceiling_seconds: Option<u64>,
    #[case] watchdog: u64,
    #[case] expected: bool,
) {
    // Both lanes here sit fifteen minutes above their requirement, so
    // the assertion over the workflows cannot distinguish a predicate
    // that compares correctly from one that ignores the allowance
    // entirely. Driving the predicate is what makes that visible.
    assert_eq!(
        ceiling_covers_watchdog_budget(ceiling_seconds, watchdog),
        expected,
        "a ceiling of {ceiling_seconds:?}s against a {watchdog}s watchdog"
    );
}

proptest::proptest! {
    /// The predicate agrees with the arithmetic over the whole domain.
    ///
    /// The bounded cases above cover the boundary; this covers the rest,
    /// as ADR 003 allows for a domain this large. `u64` watchdogs are
    /// reachable from a workflow file, so the saturating requirement is
    /// exercised here rather than assumed: a wrapping add would turn a
    /// preposterous watchdog into a small requirement that every ceiling
    /// satisfies.
    #[test]
    fn the_ceiling_predicate_agrees_with_the_requirement(
        watchdog in proptest::prelude::any::<u64>(),
        ceiling in proptest::prelude::any::<Option<u64>>(),
    ) {
        let required = required_ceiling(watchdog);
        proptest::prop_assert!(required >= watchdog);
        proptest::prop_assert_eq!(
            ceiling_covers_watchdog_budget(ceiling, watchdog),
            ceiling.is_some_and(|seconds| seconds >= required)
        );
    }

    /// A job with no ceiling never passes, whatever the watchdog.
    #[test]
    fn a_missing_ceiling_never_covers_anything(watchdog in proptest::prelude::any::<u64>()) {
        proptest::prop_assert!(!ceiling_covers_watchdog_budget(None, watchdog));
    }
}

#[rstest]
#[case::an_ordinary_watchdog(3_600, 4_500)]
#[case::zero(0, OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS)]
#[case::the_largest_representable(u64::MAX, u64::MAX)]
#[case::just_inside_the_ceiling(u64::MAX - 100, u64::MAX)]
fn the_requirement_saturates_rather_than_wrapping(#[case] watchdog: u64, #[case] expected: u64) {
    // A watchdog is parsed from a workflow file, so a value near
    // `u64::MAX` is reachable by editing one. A wrapping add would turn
    // it into a small requirement that every ceiling satisfies, which is
    // the opposite of what a preposterous watchdog should produce, and
    // the property above will not sample close enough to the boundary to
    // notice on its own.
    assert_eq!(
        required_ceiling(watchdog),
        expected,
        "the requirement for a {watchdog}s watchdog"
    );
    assert!(
        required_ceiling(watchdog) >= watchdog,
        "the requirement can never fall below the watchdog it contains"
    );
}

#[rstest]
#[case::the_documented_ceiling(90, 5_400)]
#[case::zero(0, 0)]
#[case::the_largest_representable(u64::MAX, u64::MAX)]
#[case::just_inside_the_wrap(u64::MAX / 60 + 1, u64::MAX)]
fn the_ceiling_conversion_saturates_rather_than_wrapping(
    #[case] minutes: u64,
    #[case] expected: u64,
) {
    // `timeout-minutes` is parsed from a workflow, so a value beyond
    // `u64::MAX / 60` is reachable by editing a file. Multiplying it
    // would wrap to a small number of seconds, which then fails the
    // ordering for the wrong reason: the message would report an
    // inversion where the fault is a preposterous ceiling.
    assert_eq!(
        ceiling_seconds(minutes),
        expected,
        "a ceiling of {minutes} minutes in seconds"
    );
}
