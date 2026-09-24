//! CV-005 over the closure, the publisher, and the loader beneath both.
//!
//! The boundary in `coverage_boundary.rs` says what one pull-request workflow
//! may not touch. These say where it applies: to every local workflow such a
//! lane calls, to the host as well as the credential, and to a job that
//! forwards every secret without naming any. They also hold the publisher to
//! the trunk and to a queue, and the loader to refusing a key declared twice.
//!
//! This repository calls no local reusable workflow and forwards no secret,
//! so every rule here is driven over a synthetic estate as well as the real
//! one: over the real files alone a deleted detector passes.

use std::collections::BTreeMap;

use rstest::rstest;

use crate::coverage_boundary::{
    action_of, coverage_surface_offenders, is_reachable_by_a_pull_request, pull_request_offenders,
    CODESCENE_HOST, CREDENTIAL_ENVIRONMENT_KEY,
};
use crate::coverage_reach::{local_workflow_target, reachable_workflows};
use crate::workflow_assertions::{job_named, workflows};
use crate::workflow_estate::{Workflow, WorkflowSource};
use crate::workflow_loader::parse_workflow;

/// The checkout action, whose history depth the measuring lane leaves shallow.
const CHECKOUT_ACTION: &str = "actions/checkout";

/// Parses one synthetic workflow, or panics naming it.
fn parsed(file: &str, text: &str) -> Workflow {
    match parse_workflow(WorkflowSource { file, text }) {
        Ok(workflow) => workflow,
        Err(err) => panic!("{file} must parse: {err}"),
    }
}

/// Builds a synthetic estate: its parsed workflows and their raw texts.
fn estate(files: &[(&str, &str)]) -> (Vec<Workflow>, BTreeMap<String, String>) {
    let workflows = files
        .iter()
        .map(|(file, text)| parsed(file, text))
        .collect();
    let texts = files
        .iter()
        .map(|(file, text)| ((*file).to_owned(), (*text).to_owned()))
        .collect();
    (workflows, texts)
}

/// The probe: a child declaring `workflow_call` alone, called with
/// `secrets: inherit`, curling the `CodeScene` API with the credential. Its
/// triggers make it look unreachable and its caller names neither the host nor
/// the credential, so every offence has to come back through the closure.
#[rstest]
fn a_reusable_child_is_judged_with_its_caller() {
    let (workflows, texts) = estate(&[
        (
            "parent.yml",
            "on:\n  pull_request:\njobs:\n  call:\n    uses: ./.github/workflows/child.yml\n    \
             secrets: inherit\n",
        ),
        (
            "child.yml",
            "on:\n  workflow_call:\njobs:\n  probe:\n    runs-on: ubuntu-latest\n    steps:\n      \
             - run: |\n          curl -H \"Authorization: Bearer ${{ secrets.CS_ACCESS_TOKEN }}\" \
             https://api.codescene.io/v2/projects\n",
        ),
    ]);
    let offenders = pull_request_offenders("parent.yml", &workflows, &texts);
    for expected in [
        "parent.yml:call forwards every secret".to_owned(),
        format!("child.yml: raw text references {CREDENTIAL_ENVIRONMENT_KEY}"),
        format!("child.yml: raw text references {CODESCENE_HOST}"),
    ] {
        assert!(
            offenders.iter().any(|offence| offence.contains(&expected)),
            "the closure must report {expected:?}; got {offenders:?}"
        );
    }
}

#[rstest]
fn a_local_call_to_nothing_is_reported_rather_than_skipped() {
    let (workflows, texts) = estate(&[(
        "parent.yml",
        "on: pull_request\njobs:\n  a:\n    uses: .github/workflows/gone.yml\n",
    )]);
    let offenders = pull_request_offenders("parent.yml", &workflows, &texts);
    assert!(
        offenders.iter().any(|offence| offence.contains("gone.yml")),
        "a call to a missing local workflow must be reported: {offenders:?}"
    );
}

/// A raw text that does not parse is reported, not read as forwarding nothing.
#[rstest]
fn an_unparseable_raw_text_is_reported_rather_than_read_as_clean() {
    let text = "on: pull_request\njobs:\n  a:\n    runs-on: x\n    steps:\n      - run: y\n";
    let offenders = coverage_surface_offenders(&parsed("scratch.yml", text), "jobs: [unclosed\n");
    assert!(
        offenders
            .iter()
            .any(|offence| offence.contains("raw text does not parse")),
        "a raw text the reader cannot parse must be an offence: {offenders:?}"
    );
}

/// A reached workflow whose raw text is missing is reported, not read as empty.
#[rstest]
fn a_reached_workflow_without_raw_text_is_reported() {
    let (workflows, mut texts) = estate(&[(
        "parent.yml",
        "on: pull_request\njobs:\n  a:\n    runs-on: x\n    steps:\n      - run: y\n",
    )]);
    texts.clear();
    let offenders = pull_request_offenders("parent.yml", &workflows, &texts);
    assert!(
        offenders
            .iter()
            .any(|offence| offence.contains("parent.yml: no raw text was supplied")),
        "a workflow with no raw text must be reported, not scanned as empty: {offenders:?}"
    );
}

/// A half-finished edit can produce a cycle; a recursing walk would hang.
#[rstest]
fn the_walk_stops_on_a_cycle_and_reads_each_workflow_once() {
    let (workflows, _) = estate(&[
        (
            "parent.yml",
            "on: pull_request\njobs:\n  a:\n    uses: ./.github/workflows/child.yml\n",
        ),
        (
            "child.yml",
            "on: workflow_call\njobs:\n  b:\n    uses: ./.github/workflows/parent.yml\n",
        ),
    ]);
    assert_eq!(
        reachable_workflows("parent.yml", &workflows).reached,
        ["parent.yml", "child.yml"]
    );
    assert_eq!(
        reachable_workflows("child.yml", &workflows).reached,
        ["child.yml", "parent.yml"]
    );
}

/// Local by shape, not by a list of prefixes somebody has to extend.
#[rstest]
#[case::with_a_leading_dot("./.github/workflows/child.yml", Some("child.yml"))]
#[case::without_one(".github/workflows/child.yml", Some("child.yml"))]
#[case::the_self_repository_form("$/.github/workflows/child.yml", Some("child.yml"))]
#[case::foreign("leynos/shared-actions/.github/workflows/x.yml@v1", None)]
#[case::an_action_directory("./.github/actions/setup", None)]
fn a_local_call_is_recognised_by_its_shape(#[case] uses: &str, #[case] target: Option<&str>) {
    assert_eq!(local_workflow_target(uses), target);
}

/// Neither the host nor the credential hides behind a change of case.
#[rstest]
#[case::the_host_by_another_route(
    "      - run: curl -fsS https://api.CodeScene.io/v2/projects/1\n",
    CODESCENE_HOST
)]
#[case::the_credential_in_lower_case(
    "      - run: echo ${{ secrets.cs_access_token }}\n",
    CREDENTIAL_ENVIRONMENT_KEY
)]
fn a_reference_is_found_in_any_case(#[case] step: &str, #[case] needle: &str) {
    let text = format!("on:\n  pull_request:\njobs:\n  a:\n    runs-on: x\n    steps:\n{step}");
    let offenders = coverage_surface_offenders(&parsed("scratch.yml", &text), &text);
    assert!(
        offenders
            .iter()
            .any(|offence| offence.contains(&format!("references {needle}"))),
        "a lane naming {needle} must be reported: {offenders:?}"
    );
}

#[rstest]
#[case::a_scalar("on: pull_request\n")]
#[case::a_sequence("on: [push, pull_request]\n")]
#[case::a_mapping("on:\n  pull_request:\n")]
#[case::the_boolean_key("true: pull_request\n")]
#[case::the_merge_queue("on: merge_group\n")]
#[case::a_review("on:\n  pull_request_review:\n    types: [submitted]\n")]
#[case::a_review_comment("on: [push, pull_request_review_comment]\n")]
fn every_trigger_shape_under_either_key_is_read(#[case] declaration: &str) {
    let text = format!("{declaration}jobs:\n  a:\n    runs-on: x\n    steps:\n      - run: y\n");
    assert!(
        is_reachable_by_a_pull_request(&parsed("scratch.yml", &text)),
        "{declaration:?} declares a trigger a pull request fires and must read as reachable"
    );
}

/// Both `on` keys at once is refused, since GitHub merges them and a reader
/// that picked one would be blind to the other's triggers.
#[rstest]
fn a_workflow_declaring_both_trigger_keys_is_refused() {
    let tail = "jobs:\n  a:\n    runs-on: x\n    steps:\n      - run: y\n";
    let both = format!("on: push\ntrue: pull_request\n{tail}");
    let one = format!("true: pull_request\n{tail}");
    assert!(parse_workflow(WorkflowSource {
        file: "a.yml",
        text: &one
    })
    .is_ok());
    assert!(
        parse_workflow(WorkflowSource {
            file: "a.yml",
            text: &both
        })
        .is_err(),
        "a workflow declaring `on` under both keys must be refused"
    );
}

/// `$/` resolves at the running commit, so `$/...@ref` names no workflow and
/// the walk reports it rather than following or skipping it.
#[rstest]
fn a_self_repository_call_with_a_ref_is_reported() {
    let (workflows, texts) = estate(&[
        (
            "parent.yml",
            "on: pull_request\njobs:\n  a:\n    uses: $/.github/workflows/child.yml@main\n",
        ),
        (
            "child.yml",
            "on: workflow_call\njobs:\n  b:\n    runs-on: x\n    steps:\n      - run: y\n",
        ),
    ]);
    let offenders = pull_request_offenders("parent.yml", &workflows, &texts);
    assert!(
        offenders
            .iter()
            .any(|offence| offence.contains("child.yml@main")),
        "a `$/` call carrying a ref must be reported: {offenders:?}"
    );
}

/// The loader keeps neither of two equal keys; a lane with two `runs-on` has
/// one a reviewer reads and one a runner might, and no rule may choose.
#[rstest]
fn a_key_declared_twice_is_refused() {
    let body = "on: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n";
    let tail = "    steps:\n      - run: y\n";
    let single = format!("{body}{tail}");
    let doubled = format!("{body}    runs-on: ubicloud-standard-2\n{tail}");
    assert!(parse_workflow(WorkflowSource {
        file: "a.yml",
        text: &single
    })
    .is_ok());
    assert!(
        parse_workflow(WorkflowSource {
            file: "a.yml",
            text: &doubled
        })
        .is_err(),
        "a job declaring `runs-on` twice must be refused, not resolved to one of them"
    );
}

#[rstest]
fn the_measuring_lane_checks_out_shallow(workflows: Vec<Workflow>) {
    let depths: Vec<&str> = job_named(&workflows, "build-test")
        .steps
        .iter()
        .filter(|step| action_of(step) == CHECKOUT_ACTION)
        .map(|step| step.input("fetch-depth"))
        .collect();
    assert!(
        !depths.is_empty(),
        "build-test must check the repository out"
    );
    assert!(
        depths.iter().all(|depth| matches!(*depth, "" | "1")),
        "build-test must keep the shallow default: full history was for the \
         CodeScene check, which no longer runs on a pull request; got {depths:?}"
    );
}
