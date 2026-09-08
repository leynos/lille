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
use crate::timeout_budgets::{
    ceiling_covers_watchdog_budget, ceiling_seconds, invokes_coverage, required_ceiling,
    COVERAGE_ACTION, OUTSIDE_WATCHDOG_ALLOWANCE_SECONDS,
};
use crate::workflow_assertions::{jobs, workflows};
use crate::workflow_estate::Workflow;

/// The environment variable the shared coverage action reads for its
/// wall-clock cap on one `cargo` invocation.
const WATCHDOG_VARIABLE: &str = "RUN_RUST_CARGO_WAIT_TIMEOUT";

/// The watchdog every coverage job must set, in seconds, as
/// `docs/developers-guide.md` records it. Pinned rather than merely
/// checked for shape: the ordering below holds for a wide range of
/// values, so nothing else would notice this one drifting.
const REQUIRED_WATCHDOG_SECONDS: u64 = 3_600;

/// The ceiling every coverage job must declare, in minutes, likewise
/// from the guide. It is the requirement plus the fifteen minutes of
/// slack the guide asks for, so the two arithmetic statements meet here.
const REQUIRED_CEILING_MINUTES: u64 = 90;

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
            let required = required_ceiling(watchdog);
            let ceiling = job.timeout_minutes.map(ceiling_seconds);
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
