//! CV-005: only `main` writes persistent coverage state.
//!
//! A pull-request lane measures coverage and compares it with the ratcheted
//! baseline `main` produced. It does not publish the report, call the
//! `CodeScene` action, run a `cs-coverage` command, or hold the credential
//! either of those needs. The check step that used to sit in `ci.yml` is why
//! a `CodeScene` outage or a token change could redden a pull request that had
//! touched nothing to do with coverage.
//!
//! The readers live in `coverage_boundary.rs` and take a parsed workflow with
//! its raw text, so the offences can be driven here as synthetic documents.
//! That matters: over this repository's own files a reader that answered
//! nothing agrees with a correct one exactly, and the rule would pass with
//! every detector deleted.

use std::collections::BTreeMap;

use rstest::rstest;

use crate::coverage_boundary::{
    action_of, coverage_surface_offenders, declines_the_generated_report_archive,
    is_reachable_by_a_pull_request, publishes_the_coverage_report, pull_request_offenders,
    GENERATE_COVERAGE_ACTION, PUBLICATION_OPT_OUT_INPUT, PUBLICATION_OPT_OUT_VALUE,
    UPLOAD_COVERAGE_ACTION,
};
use crate::workflow_assertions::{job_named, workflows};
use crate::workflow_estate::{Workflow, WorkflowSource};
use crate::workflow_loader::{parse_workflow, repository_workflow_text};

/// The lane that owns the upload, and is therefore exempt.
const PUBLISHER_WORKFLOW: &str = "coverage-main.yml";

/// Returns one workflow's raw text as the repository stores it, or panics.
///
/// Read through the loader's directory capability rather than the ambient
/// filesystem: the one ambient step in this suite lives there, and a contract
/// that reached past it would be the second.
fn raw_text(file: &str) -> String {
    match repository_workflow_text(file) {
        Ok(text) => text,
        Err(err) => panic!("{file} must be readable: {err}"),
    }
}

/// Builds a one-job workflow from a step body, for driving the readers.
fn synthetic(steps: &str) -> Workflow {
    let text = format!(
        "on:\n  pull_request:\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n{steps}"
    );
    match parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text: &text,
    }) {
        Ok(workflow) => workflow,
        Err(err) => panic!("the synthetic workflow must parse: {err}"),
    }
}

/// The boundary over every lane a pull request can start and every local
/// workflow such a lane calls, since a reusable child declares
/// `workflow_call` alone and would otherwise never be asked.
#[rstest]
fn no_pull_request_workflow_touches_the_coverage_publication_surface(workflows: Vec<Workflow>) {
    let texts: BTreeMap<String, String> = workflows
        .iter()
        .map(|workflow| (workflow.file.clone(), raw_text(&workflow.file)))
        .collect();
    let offenders: Vec<String> = workflows
        .iter()
        .filter(|workflow| workflow.file != PUBLISHER_WORKFLOW)
        .filter(|workflow| is_reachable_by_a_pull_request(workflow))
        .flat_map(|workflow| pull_request_offenders(&workflow.file, &workflows, &texts))
        .collect();
    assert!(
        offenders.is_empty(),
        "a lane a pull request can reach, and every local workflow it calls, must \
         not publish the report, call the CodeScene action, run a coverage command, \
         name the CodeScene host, forward every secret, or hold the credential; \
         `{PUBLISHER_WORKFLOW}` owns the surface: {offenders:?}"
    );
}

#[rstest]
fn the_pull_request_lane_measures_against_the_ratchet_and_keeps_the_report(
    workflows: Vec<Workflow>,
) {
    let job = job_named(&workflows, "build-test");
    let step = job
        .steps
        .iter()
        .find(|step| action_of(step) == GENERATE_COVERAGE_ACTION)
        .expect("build-test must generate coverage");
    assert_eq!(
        step.input("with-ratchet"),
        "true",
        "the pull-request lane must compare against the baseline `{PUBLISHER_WORKFLOW}` \
         wrote; without it the lane measures and reports nothing"
    );
    assert!(
        declines_the_generated_report_archive(step),
        "the pull-request lane must pass `{PUBLICATION_OPT_OUT_INPUT}: \
         {PUBLICATION_OPT_OUT_VALUE}`. The action archives the report under a step \
         of its own, which no scanner over this file can see, so the opt-out is \
         the only place the boundary is observable"
    );
}

#[rstest]
fn the_publisher_keeps_the_upload_this_boundary_moved_to_it(workflows: Vec<Workflow>) {
    let publisher = workflows
        .iter()
        .find(|workflow| workflow.file == PUBLISHER_WORKFLOW)
        .unwrap_or_else(|| panic!("the estate must define {PUBLISHER_WORKFLOW}"));
    assert!(
        !is_reachable_by_a_pull_request(publisher),
        "{PUBLISHER_WORKFLOW} must not be reachable by a pull request, or moving the \
         upload into it moves nothing"
    );
    let uploads = publisher
        .jobs
        .iter()
        .flat_map(|job| &job.steps)
        .any(|step| action_of(step) == UPLOAD_COVERAGE_ACTION);
    assert!(
        uploads,
        "{PUBLISHER_WORKFLOW} must keep the CodeScene upload; a rule that only \
         forbids it elsewhere is satisfied by deleting it everywhere"
    );
}

/// Each forbidden element, added back one at a time.
#[rstest]
#[case::the_codescene_action(
    "      - uses: leynos/shared-actions/.github/actions/upload-codescene-coverage@abc\n",
    "invokes the CodeScene coverage action"
)]
#[case::a_coverage_command(
    "      - run: cs-coverage check --format lcov\n",
    "runs a cs-coverage command"
)]
#[case::an_artefact_upload(
    "      - uses: actions/upload-artifact@abc\n        with:\n          path: lcov.info\n",
    "publishes the coverage report as an artefact"
)]
#[case::an_unnamed_artefact_upload(
    "      - uses: actions/upload-artifact@abc\n",
    "publishes the coverage report as an artefact"
)]
#[case::a_coverage_call_that_keeps_its_archive(
    "      - uses: leynos/shared-actions/.github/actions/generate-coverage@abc\n",
    "without declining its own archive"
)]
fn each_forbidden_element_is_reported(#[case] step: &str, #[case] expected: &str) {
    let offenders = coverage_surface_offenders(&synthetic(step), "");
    assert!(
        offenders.iter().any(|offence| offence.contains(expected)),
        "a lane declaring this step must be reported as {expected:?}; got {offenders:?}"
    );
}

#[rstest]
fn the_credential_is_found_in_raw_text_a_parser_would_drop() {
    let clean = synthetic("      - run: echo hello\n");
    assert!(
        coverage_surface_offenders(&clean, "").is_empty(),
        "an ordinary lane must produce no offence, or the detectors accuse rather \
         than discriminate"
    );
    let offenders = coverage_surface_offenders(&clean, "# see CS_ACCESS_TOKEN in main\n");
    assert!(
        offenders.iter().any(|offence| offence.contains("raw text")),
        "the credential must be found in a comment too: a parsed-value scan alone \
         misses a reference the file still carries; got {offenders:?}"
    );
}

#[rstest]
fn a_compliant_coverage_call_is_not_accused() {
    let offenders = coverage_surface_offenders(
        &synthetic(
            "      - uses: leynos/shared-actions/.github/actions/generate-coverage@abc\n\
             \x20       with:\n          with-ratchet: 'true'\n          publish-artefact: 'false'\n",
        ),
        "",
    );
    assert!(
        offenders.is_empty(),
        "the shape this contract asks for must itself pass: {offenders:?}"
    );
}

#[rstest]
fn a_lookalike_action_is_not_the_action(#[values("-legacy", "-v2")] suffix: &str) {
    let step = format!("      - uses: {UPLOAD_COVERAGE_ACTION}{suffix}@abc\n");
    assert!(
        coverage_surface_offenders(&synthetic(&step), "").is_empty(),
        "`{UPLOAD_COVERAGE_ACTION}{suffix}` is a different action; splitting on the \
         version separator rather than matching a prefix is what tells them apart"
    );
}

#[rstest]
fn an_artefact_step_that_names_another_path_is_not_an_offence() {
    let step = "      - uses: actions/upload-artifact@abc\n        with:\n          path: dist/\n";
    let offenders = coverage_surface_offenders(&synthetic(step), "");
    assert!(
        offenders.is_empty(),
        "uploading something other than the report is allowed: {offenders:?}"
    );
    let workflow = synthetic(step);
    let Some(first) = workflow.jobs.first().and_then(|job| job.steps.first()) else {
        panic!("the synthetic workflow must declare one step")
    };
    assert!(
        !publishes_the_coverage_report(first),
        "the reader must answer for the step as well as for the whole workflow"
    );
}
