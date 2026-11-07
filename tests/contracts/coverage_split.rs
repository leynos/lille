//! The main-branch coverage split: measure on a hosted runner, publish the
//! ratchet baseline and warm the compiler cache on Ubicloud.
//!
//! A hosted runner writes GitHub's cache store, which the Ubicloud
//! pull-request lanes never read. Each rule here holds one seam of the
//! hand-off: where the figures leave the hosted job, where they are saved, and
//! what the warm-up compiles.

use rstest::rstest;

use crate::shared_action;
use crate::workflow_assertions::{assert_input, job_named, step_using, workflows};
use crate::workflow_estate::{Workflow, BASELINE_JOB_ID, BASELINE_LABEL, HOSTED_COVERAGE_JOB_ID};

/// The baseline publisher compiles nothing, so it takes the smallest Ubicloud
/// size, and it must read the cache store the pull-request lanes read.
#[rstest]
fn the_baseline_publisher_sits_on_the_smallest_ubicloud_runner(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, BASELINE_JOB_ID);
    assert_eq!(
        job.runs_on.owned_label(),
        Some(BASELINE_LABEL),
        "`{BASELINE_JOB_ID}` must run on `{BASELINE_LABEL}`; it declares {}",
        job.runs_on
    );
    let timeout = job.timeout_minutes.unwrap_or_else(|| {
        panic!("`{BASELINE_JOB_ID}` bills by the minute and must declare timeout-minutes")
    });
    assert!(
        (1..=30).contains(&timeout),
        "`{BASELINE_JOB_ID}` timeout-minutes {timeout} must lie between 1 and 30"
    );
}

/// The hosted job must hand its figures on under the name the publisher reads,
/// and must not save a baseline into a store the lanes cannot see.
#[rstest]
fn the_hosted_job_hands_its_baseline_to_the_publisher_by_artefact(workflows: Vec<Workflow>) {
    let generator = step_using(
        job_named(&workflows, HOSTED_COVERAGE_JOB_ID),
        &shared_action("generate-coverage"),
    );
    let publisher = step_using(
        job_named(&workflows, BASELINE_JOB_ID),
        &shared_action("publish-coverage-baseline"),
    );
    assert_input(
        HOSTED_COVERAGE_JOB_ID,
        generator,
        "publish-baseline",
        "never",
    );
    let handed_over = generator.input("baseline-artefact-name");
    assert!(
        !handed_over.is_empty(),
        "`{HOSTED_COVERAGE_JOB_ID}` must name the artefact that carries its baseline"
    );
    assert_input(BASELINE_JOB_ID, publisher, "artefact-name", handed_over);
}

/// The warm-up compiles and stops: a test run would make it a second coverage
/// measurement, and a build with other flags would warm entries nothing reads.
#[rstest]
fn the_warmup_compiles_the_lane_build_without_running_it(workflows: Vec<Workflow>) {
    let warmup = step_using(
        job_named(&workflows, "coverage-warmup"),
        &shared_action("generate-coverage"),
    );
    let lane = step_using(
        job_named(&workflows, "build-test"),
        &shared_action("generate-coverage"),
    );
    assert_input("coverage-warmup", warmup, "build-only", "true");
    for key in ["all-features", "all-targets", "use-cargo-nextest"] {
        assert_input("coverage-warmup", warmup, key, lane.input(key));
    }
}
