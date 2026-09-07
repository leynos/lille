//! Timeout-ordering contracts over the coverage lanes.
//!
//! Four timers can end a test run, each set somewhere different, and they
//! only work if each sits above the one inside it. Two of the four exist
//! here. The coverage action runs `cargo llvm-cov` with plain `cargo test`
//! rather than nextest, so there is no per-test `slow-timeout` and no
//! whole-run `global-timeout`; what remains is the shared action's
//! wall-clock watchdog on the `cargo` invocation, and the job's own
//! ceiling above it.
//!
//! The watchdog is the tier nobody expects. It belongs to the shared
//! `generate-coverage` action, defaults to 1,800 s, and nothing in this
//! repository would mention it if a job stopped setting it. That default
//! is well under the 1,728 s this repository's coverage step has already
//! taken, so inheriting it is not a theoretical loss.
//!
//! See "Test timeouts: the two tiers this repository has" in
//! `docs/developers-guide.md`, and the canonical wording in
//! `leynos/shared-actions`' `generate-coverage` README.

use rstest::rstest;

use crate::shared_action;
use crate::workflow_assertions::{jobs, workflows};
use crate::workflow_estate::Workflow;

/// The environment variable the shared coverage action reads for its
/// wall-clock cap on one `cargo` invocation.
const WATCHDOG_VARIABLE: &str = "RUN_RUST_CARGO_WAIT_TIMEOUT";

/// Everything in a coverage job that is not the `cargo` invocation the
/// watchdog bounds: checkout, toolchain setup, cache restore, and whatever
/// follows the coverage step. The job timer covers it; the watchdog does
/// not.
///
/// Measured from the worst of several runs rather than one. Across fifteen
/// successful `ci.yml` runs the widest gap between the coverage step and
/// its job was 859 s on run 33830336409; across twenty of
/// `coverage-main.yml` it was 286 s on run 31892219565. Fifteen minutes
/// covers the worse of those, and none of those runs was genuinely cold.
const OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS: u64 = 15 * 60;

/// The name of the coverage action, without its owner or pin.
const COVERAGE_ACTION: &str = "generate-coverage";

/// The watchdog every coverage job must set, in seconds, as
/// `docs/developers-guide.md` records it. Pinned rather than merely
/// checked for shape: the ordering below holds for a wide range of
/// values, so nothing else would notice this one drifting.
const REQUIRED_WATCHDOG_SECONDS: u64 = 3_600;

/// The ceiling every coverage job must declare, in minutes, likewise
/// from the guide. It is the requirement plus the fifteen minutes of
/// slack the guide asks for, so the two arithmetic statements meet here.
const REQUIRED_CEILING_MINUTES: u64 = 90;

/// Returns whether a step invokes the coverage action.
///
/// The `uses` value is the coordinate, an `@`, and a pin. Matching the
/// coordinate as a prefix of the whole string would also match a
/// sibling action whose name merely begins with this one's, such as
/// `generate-coverage-variant`, and that action has no watchdog of its
/// own for the assertions below to be about.
fn invokes_coverage(uses: &str, coordinate: &str) -> bool {
    uses.split('@').next().unwrap_or(uses) == coordinate
}

/// Returns every job that invokes the coverage action, with its file.
fn coverage_jobs(workflows: &[Workflow]) -> Vec<(String, crate::workflow_model::Job)> {
    let coordinate = shared_action(COVERAGE_ACTION);
    jobs(workflows)
        .into_iter()
        .filter(|(_, job)| {
            job.steps
                .iter()
                .any(|step| invokes_coverage(&step.uses, &coordinate))
        })
        .collect()
}

/// Returns whether a ceiling contains a watchdog and the work around it.
///
/// Extracted so the decision is one named thing and the three messages
/// below are only messages. `None` is a job that declares no ceiling at
/// all, which is not a smaller number but a different failure: GitHub's
/// six-hour default applies and nothing in the workflow says so.
fn ceiling_covers_watchdog_budget(ceiling_seconds: Option<u64>, watchdog: u64) -> bool {
    ceiling_seconds.is_some_and(|seconds| seconds >= watchdog + OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS)
}

#[rstest]
fn some_job_invokes_the_coverage_action(workflows: Vec<Workflow>) {
    // Without this, a rename or a repin that stopped the coordinate
    // matching would turn every assertion below into a vacuous pass over
    // an empty list, and the loss would look exactly like success.
    let found = coverage_jobs(&workflows);
    assert!(
        !found.is_empty(),
        "no job invokes {}; either coverage moved or this contract stopped \
         recognizing it",
        shared_action(COVERAGE_ACTION)
    );
}

#[rstest]
fn every_coverage_job_sets_the_watchdog_explicitly(workflows: Vec<Workflow>) {
    // The action's default is 1,800 s and nothing here names it, so a job
    // that stopped setting the variable would inherit a budget this
    // repository has not chosen and does not mention. The coverage step has
    // already taken 1,728 s, so that default is not far above the work.
    let missing: Vec<String> = coverage_jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| job.env(WATCHDOG_VARIABLE).is_empty())
        .map(|(file, job)| format!("{file}:{}", job.id))
        .collect();
    assert!(
        missing.is_empty(),
        "these coverage jobs do not set {WATCHDOG_VARIABLE} and so inherit the \
         shared action's undocumented 1,800 s default: {missing:?}"
    );
}

#[rstest]
fn the_watchdog_is_a_whole_number_of_seconds(workflows: Vec<Workflow>) {
    // The action reads the variable as seconds. A value carrying a unit,
    // or a decimal, would be read as something other than what its author
    // meant, and the job would not fail until a run overran.
    let malformed: Vec<String> = coverage_jobs(&workflows)
        .into_iter()
        .filter_map(|(file, job)| {
            let raw = job.env(WATCHDOG_VARIABLE);
            match raw.parse::<u64>() {
                Ok(value) if value > 0 => None,
                _ => Some(format!("{file}:{}: {WATCHDOG_VARIABLE}={raw:?}", job.id)),
            }
        })
        .collect();
    assert!(
        malformed.is_empty(),
        "{WATCHDOG_VARIABLE} is a count of seconds; these are not: {malformed:?}"
    );
}

#[rstest]
fn the_job_ceiling_covers_the_watchdog_and_the_work_around_it(workflows: Vec<Workflow>) {
    // The two clocks do not start together. The job timer starts when the
    // job starts, before the checkout and the toolchain setup, and it is
    // still running through whatever follows coverage. The watchdog starts
    // when `cargo` does. A ceiling merely above the watchdog still cancels
    // the job before the watchdog can report the overrun, and a
    // cancellation discards the log that would have explained it.
    let inverted: Vec<String> = coverage_jobs(&workflows)
        .into_iter()
        .filter_map(|(file, job)| {
            let watchdog: u64 = job.env(WATCHDOG_VARIABLE).parse().ok()?;
            let required = watchdog + OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS;
            let ceiling = job.timeout_minutes.map(|minutes| minutes * 60);
            if ceiling_covers_watchdog_budget(ceiling, watchdog) {
                return None;
            }
            let detail = ceiling.map_or_else(
                || {
                    format!(
                        "runs cargo under a {watchdog}s watchdog in a job with \
                         no timeout-minutes; the outermost tier is missing and \
                         GitHub's six-hour default applies"
                    )
                },
                |seconds| {
                    format!(
                        "timeout-minutes gives {seconds}s, below the \
                         {required}s needed to cover a {watchdog}s watchdog \
                         plus {OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS}s of \
                         measured work outside it"
                    )
                },
            );
            Some(format!("{file}:{}: {detail}", job.id))
        })
        .collect();
    assert!(
        inverted.is_empty(),
        "these coverage jobs would be cancelled before the watchdog could \
         report an overrun: {inverted:?}"
    );
}

#[rstest]
fn the_nextest_tiers_are_absent_rather_than_unset(workflows: Vec<Workflow>) {
    // The canonical section has four tiers because nextest contributes two
    // of them. This repository runs `cargo test` under `cargo llvm-cov`
    // instead, so it has neither, and there is no `.config/nextest.toml`
    // for anyone to have set them in.
    //
    // Turning nextest on would introduce both tiers at once, unbounded: no
    // per-test allowance and no whole-run budget, underneath a watchdog
    // sized for neither. That is a change to the timeout shape, so it must
    // arrive with the guide section updated, and this is what makes it.
    let coordinate = shared_action(COVERAGE_ACTION);
    let enabled: Vec<String> = jobs(&workflows)
        .into_iter()
        .flat_map(|(file, job)| {
            let id = job.id.clone();
            job.steps
                .iter()
                .filter(|step| invokes_coverage(&step.uses, &coordinate))
                .filter(|step| step.input("use-cargo-nextest") != "false")
                .map(|step| {
                    format!(
                        "{file}:{id}: use-cargo-nextest={:?}",
                        step.input("use-cargo-nextest")
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        enabled.is_empty(),
        "these coverage steps enable nextest, which adds a per-test and a \
         whole-run budget with nothing setting either; introduce both in \
         `.config/nextest.toml` and update the developers' guide in the same \
         change: {enabled:?}"
    );
}

#[rstest]
fn every_coverage_job_carries_the_documented_watchdog(workflows: Vec<Workflow>) {
    // The ordering above holds for a wide range of watchdogs, so on its
    // own it would let this one drift away from the developers' guide
    // without failing anything. The value and its ceiling are one
    // statement, and the guide is where that statement lives.
    let wrong: Vec<String> = coverage_jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| job.env(WATCHDOG_VARIABLE) != REQUIRED_WATCHDOG_SECONDS.to_string())
        .map(|(file, job)| {
            format!(
                "{file}:{}: {WATCHDOG_VARIABLE}={:?}",
                job.id,
                job.env(WATCHDOG_VARIABLE)
            )
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "these coverage jobs do not set the documented \
         {REQUIRED_WATCHDOG_SECONDS}s watchdog: {wrong:?}; change the \
         developers' guide with them or change them back"
    );
}

#[rstest]
fn every_coverage_job_carries_the_documented_ceiling(workflows: Vec<Workflow>) {
    // Likewise for the outermost tier. The derived check accepts any
    // ceiling at or above 75 minutes; the guide states 90, which is that
    // requirement plus the fifteen minutes of slack it asks for.
    let wrong: Vec<String> = coverage_jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| job.timeout_minutes != Some(REQUIRED_CEILING_MINUTES))
        .map(|(file, job)| format!("{file}:{}: {:?}", job.id, job.timeout_minutes))
        .collect();
    assert!(
        wrong.is_empty(),
        "these coverage jobs do not carry the documented \
         {REQUIRED_CEILING_MINUTES}-minute ceiling: {wrong:?}"
    );
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
