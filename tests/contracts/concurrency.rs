//! Contracts cancelling superseded pull-request runs.
//!
//! Every push to a pull request starts a fresh run of each gate, and the run
//! already in flight is answering a question about a commit nobody will
//! merge. Left alone it holds a runner until it finishes, so the branch pays
//! twice for one answer. A concurrency group keyed on the pull request makes
//! the newer run cancel the older one.
//!
//! Cancellation has to stay conditioned on the event. A literal
//! `cancel-in-progress: true` would also cancel a push to `main`, a schedule,
//! and a dispatch, none of which has a successor waiting: the run that writes
//! the warm cache on `main` would be killed by the next merge, and the
//! coverage history would gain holes. The condition is part of the contract
//! rather than an implementation detail, and
//! `cancellation_is_conditioned_on_the_pull_request_event` fails on the
//! literal.
//!
//! The group also has to distinguish one pull request from another. A group
//! derived from `github.run_id` is unique per run and so cancels nothing,
//! while a constant group would let one branch cancel another's gates.
//!
//! The run id has exactly one legitimate place: the fallback arm after the
//! pull request number, which only a push or a dispatch reaches. Those runs
//! have no predecessor to cancel, and falling back to the ref instead would
//! let a third push or dispatch replace a pending second run that was meant
//! to complete.
//!
//! Only `pull_request` is in scope. A `pull_request_target` workflow runs
//! against the base repository to carry a token, and the one that uses it
//! here automates pull-request housekeeping rather than building; cancelling
//! an auto-merge mid-flight is a hazard with no minutes to win.

use rstest::rstest;

use crate::workflow_assertions::workflows;
use crate::workflow_estate::Workflow;

/// The exact `cancel-in-progress` value every pull-request workflow carries.
///
/// The comparison is against this one string rather than a substring search,
/// which is what makes a literal `true` fail: rendered as written it is
/// `"true"`, and that never equals this.
const CANCEL_EXPRESSION: &str = "${{ github.event_name == 'pull_request' }}";

/// The trigger that puts a workflow in scope.
const PULL_REQUEST: &str = "pull_request";

/// Expressions unique to a single run.
///
/// A group built from one of these can never match another run, so it queues
/// nothing and cancels nothing while reading exactly like a concurrency
/// control.
const RUN_UNIQUE_EXPRESSIONS: [&str; 4] = [
    "github.run_id",
    "github.run_number",
    "github.run_attempt",
    "github.sha",
];

/// The one position a run-unique value may take in a pull-request group.
///
/// The arm after `github.event.pull_request.number` is evaluated only when
/// there is no pull request, so a run id there cancels nothing a pull request
/// needs cancelled.
const RUN_UNIQUE_FALLBACK: &str = " || github.run_id }}";

/// Returns a group with its run-unique fallback arm removed, if it has one.
fn without_the_fallback(group: &str) -> String {
    group.replacen(RUN_UNIQUE_FALLBACK, " }}", 1)
}

/// Expressions that differ between two pull requests.
///
/// A group naming none of them is shared by every branch, so one pull
/// request's push would cancel another's gates.
const PER_PULL_REQUEST_EXPRESSIONS: [&str; 3] = [
    "github.event.pull_request.number",
    "github.head_ref",
    "github.ref",
];

/// Workflows known to start on `pull_request`.
///
/// Discovery below reads the estate, so a new workflow is covered the day it
/// lands. A dynamic list that silently empties would turn every contract here
/// into a vacuous pass, and this names the floor discovery must still reach.
const KNOWN_PULL_REQUEST_WORKFLOWS: [&str; 1] = ["ci.yml"];

/// Returns every workflow a pull request can start.
fn pull_request_workflows(workflows: &[Workflow]) -> Vec<&Workflow> {
    workflows
        .iter()
        .filter(|workflow| workflow.has_trigger(PULL_REQUEST))
        .collect()
}

#[rstest]
fn discovery_still_finds_the_known_pull_request_workflows(workflows: Vec<Workflow>) {
    // Every contract below iterates a list built by reading the estate. If
    // that read broke, or the `on` key changed shape, the list would empty
    // and each contract would report as passed having asserted nothing.
    let discovered: Vec<&str> = pull_request_workflows(&workflows)
        .iter()
        .map(|workflow| workflow.file.as_str())
        .collect();
    for expected in KNOWN_PULL_REQUEST_WORKFLOWS {
        assert!(
            discovered.contains(&expected),
            "`{expected}` starts on `{PULL_REQUEST}` but discovery missed it; \
             the contracts below would pass without asserting anything about it"
        );
    }
}

#[rstest]
fn every_pull_request_workflow_declares_a_concurrency_group(workflows: Vec<Workflow>) {
    // Without a group, every push to the branch leaves its predecessor
    // running to completion on a paid runner.
    for workflow in pull_request_workflows(&workflows) {
        let group = workflow
            .concurrency
            .as_ref()
            .map_or("", |declared| declared.group.as_str());
        assert!(
            !group.trim().is_empty(),
            "`{}` starts on `{PULL_REQUEST}` and must declare `concurrency.group`; \
             without it a superseded run holds a runner until it finishes",
            workflow.file
        );
    }
}

#[rstest]
fn the_concurrency_group_is_not_unique_to_one_run(workflows: Vec<Workflow>) {
    // A group built from the run identifier or the commit SHA matches no
    // other run, so it cancels nothing while reading as a concurrency
    // control.
    for workflow in pull_request_workflows(&workflows) {
        let group = workflow
            .concurrency
            .as_ref()
            .map_or(String::new(), |declared| {
                without_the_fallback(&declared.group)
            });
        for expression in RUN_UNIQUE_EXPRESSIONS {
            assert!(
                !group.contains(expression),
                "`{}` builds its concurrency group from `{expression}`, which is \
                 unique to one run; the group would never match a superseded run \
                 and would cancel nothing",
                workflow.file
            );
        }
    }
}

#[rstest]
fn the_concurrency_group_distinguishes_one_pull_request_from_another(workflows: Vec<Workflow>) {
    // A constant group would put every open pull request in one queue, and
    // the first push anywhere would cancel the gates running everywhere else.
    for workflow in pull_request_workflows(&workflows) {
        let group = workflow
            .concurrency
            .as_ref()
            .map_or("", |declared| declared.group.as_str());
        assert!(
            PER_PULL_REQUEST_EXPRESSIONS
                .iter()
                .any(|expression| group.contains(expression)),
            "`{}` must key its concurrency group on the pull request, by naming \
             one of {PER_PULL_REQUEST_EXPRESSIONS:?}; a group shared by every \
             branch would cancel unrelated pull requests",
            workflow.file
        );
    }
}

#[rstest]
fn outside_a_pull_request_the_group_falls_back_to_the_run(workflows: Vec<Workflow>) {
    // A fallback to the ref puts every push to a branch, and every dispatch,
    // in one queue, where a third trigger replaces a pending second run.
    for workflow in pull_request_workflows(&workflows) {
        let group = workflow
            .concurrency
            .as_ref()
            .map_or("", |declared| declared.group.as_str());
        assert!(
            group.ends_with(RUN_UNIQUE_FALLBACK),
            "`{}` must end its concurrency group with `{RUN_UNIQUE_FALLBACK}`, so a \
             run outside a pull request is its own group; got `{group}`",
            workflow.file
        );
    }
}

#[rstest]
fn cancellation_is_conditioned_on_the_pull_request_event(workflows: Vec<Workflow>) {
    // A literal `true` reads as a stricter setting and is a regression: it
    // would cancel the run on `main` that writes the warm cache and records
    // coverage, which no later run repeats.
    for workflow in pull_request_workflows(&workflows) {
        let declared = workflow
            .concurrency
            .as_ref()
            .map_or("", |found| found.cancel_in_progress.as_str());
        assert_eq!(
            declared, CANCEL_EXPRESSION,
            "`{}` must set `cancel-in-progress` to `{CANCEL_EXPRESSION}`; a missing \
             value leaves superseded runs in flight, and a literal `true` also \
             cancels pushes to `main`, schedules, and dispatches",
            workflow.file
        );
    }
}
