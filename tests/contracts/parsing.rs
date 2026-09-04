//! Parser contracts over the workflow loader.
//!
//! These read no workflow file in the repository. Their subject is the loader
//! itself: that it rejects a document whose shape the runner would reject, and
//! accepts every shape the runner accepts. A loader that silently defaulted a
//! mistyped field would let a broken workflow satisfy every rule in the other
//! contract modules.

use camino::Utf8Path;
use rstest::rstest;

use crate::workflow_estate::WorkflowSource;
use crate::workflow_loader::{load_workflows_in, parse_workflow};

#[rstest]
#[case::not_a_workflow("scratch.yml", "steps: []")]
#[case::mistyped_runner("scratch.yml", "jobs:\n  a:\n    runs-on: {group: [g]}\n")]
#[case::mistyped_runner_label("scratch.yml", "jobs:\n  a:\n    runs-on: [a, [b]]\n")]
#[case::groupless_runner_mapping("scratch.yml", "jobs:\n  a:\n    runs-on: {labels: [a]}\n")]
#[case::placeless_job("scratch.yml", "jobs:\n  a:\n    steps: []\n")]
#[case::mistyped_steps("scratch.yml", "jobs:\n  a:\n    runs-on: x\n    steps: nope\n")]
#[case::empty_step(
    "scratch.yml",
    "jobs:\n  a:\n    runs-on: x\n    steps:\n      - name: n\n"
)]
#[case::mistyped_input(
    "scratch.yml",
    "jobs:\n  a:\n    runs-on: x\n    steps:\n      - uses: u\n        with:\n          k: [1]\n"
)]
// GitHub Actions runs a step either as an action or as a script, never both.
#[case::dual_mode_step(
    "scratch.yml",
    "jobs:\n  a:\n    runs-on: x\n    steps:\n      - uses: u\n        run: echo hi\n"
)]
// A job calls a reusable workflow or runs its own steps on a runner it names.
// GitHub Actions rejects either mixture.
#[case::reusable_job_with_a_runner(
    "scratch.yml",
    "jobs:\n  a:\n    uses: o/r/.github/workflows/w.yml@v1\n    runs-on: x\n"
)]
#[case::reusable_job_with_steps(
    "scratch.yml",
    "jobs:\n  a:\n    uses: o/r/.github/workflows/w.yml@v1\n    steps: []\n"
)]
fn a_malformed_workflow_is_an_error_not_a_default(#[case] file: &str, #[case] text: &str) {
    let outcome = parse_workflow(WorkflowSource { file, text });
    assert!(
        outcome.is_err(),
        "a workflow of unexpected shape must be rejected, not silently defaulted"
    );
}

/// Every `runs-on` shape GitHub Actions accepts must parse, not just the
/// scalar one: rejecting a label list or a runner group would fail a valid
/// workflow rather than the workflow a contract is meant to catch.
#[rstest]
#[case::single_label("runs-on: ubuntu-latest\n", &["ubuntu-latest"])]
#[case::label_list("runs-on: [self-hosted, linux]\n", &["self-hosted", "linux"])]
#[case::group_only("runs-on:\n      group: ubuntu-runners\n", &[])]
#[case::group_and_labels(
    "runs-on:\n      group: ubuntu-runners\n      labels: [ubuntu-20.04-16core]\n",
    &["ubuntu-20.04-16core"]
)]
fn every_valid_runs_on_shape_parses(#[case] runs_on: &str, #[case] expected: &[&str]) {
    let text = format!("on: push\njobs:\n  a:\n    {runs_on}    steps: []\n");
    let workflow = parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text: &text,
    })
    .unwrap_or_else(|err| panic!("`{runs_on}` must parse: {err}"));
    let job = workflow
        .jobs
        .first()
        .unwrap_or_else(|| panic!("`{runs_on}` must yield a job"));
    assert_eq!(job.runs_on.labels(), expected);
    assert!(
        job.runs_on.names_a_runner(),
        "`{runs_on}` names a runner and must say so"
    );
}

#[rstest]
fn an_unreadable_workflow_directory_is_reported() {
    let missing = Utf8Path::new("this/directory/does/not/exist");
    let outcome = load_workflows_in(missing);
    assert!(
        outcome.is_err(),
        "an unreadable workflow directory must surface as an error"
    );
}
