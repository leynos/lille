//! Parser contracts for the step `id` and `env` fields.
//!
//! The compiler-cache contracts read the statistics step through its `env` and
//! find `setup-rust` through its `id`, so a loader that defaulted a mistyped
//! value, or confused an absent key with an empty one, would let a broken step
//! satisfy them. These read no workflow file in the repository.

use rstest::rstest;

use crate::workflow_estate::{Workflow, WorkflowSource};
use crate::workflow_loader::parse_workflow;
use crate::workflow_model::Step;

/// Parses a one-job workflow whose only step is `step_yaml`, indented as a step.
fn parse_with_step(step_yaml: &str) -> Result<Workflow, String> {
    let text = format!("on: push\njobs:\n  a:\n    runs-on: x\n    steps:\n{step_yaml}");
    parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text: &text,
    })
    .map_err(|err| err.to_string())
}

/// Returns the single step of the single job of a parsed workflow.
fn only_step(workflow: &Workflow) -> &Step {
    let [job] = workflow.jobs.as_slice() else {
        panic!("the workflow must declare one job")
    };
    let [step] = job.steps.as_slice() else {
        panic!("the job must declare one step")
    };
    step
}

/// A malformed `env` is refused for its own reason, not read as absent.
#[rstest]
#[case::not_a_mapping("      - run: y\n        env: nope\n", "`env` must be a mapping")]
#[case::a_sequence("      - run: y\n        env: [a]\n", "`env` must be a mapping")]
#[case::a_non_string_key(
    "      - run: y\n        env:\n          1: a\n",
    "every `env` key must be a string"
)]
#[case::a_sequence_value(
    "      - run: y\n        env:\n          K: [1]\n",
    "input `K` must be a scalar"
)]
#[case::a_mapping_value(
    "      - run: y\n        env:\n          K: {a: b}\n",
    "input `K` must be a scalar"
)]
fn a_malformed_step_env_is_an_error_not_a_default(#[case] step: &str, #[case] reason: &str) {
    let Err(err) = parse_with_step(step) else {
        panic!("a malformed `env` must be rejected, not silently defaulted")
    };
    assert!(
        err.contains(reason),
        "the step must be refused because {reason:?}, not for another reason; got {err}"
    );
}

/// A step's `env` is kept as written, expressions included, so a contract can
/// assert the exact `${{ steps.x.outputs.y }}` a report is handed.
#[rstest]
fn a_populated_step_env_is_kept_as_written() {
    let workflow = parse_with_step(concat!(
        "      - id: report\n        run: y\n        env:\n",
        "          SCCACHE_STATUS: ${{ steps.setup-rust.outputs.sccache-status }}\n",
        "          RETRIES: 3\n          EMPTY: ''\n",
    ))
    .unwrap_or_else(|err| panic!("a populated `env` must parse: {err}"));
    let step = only_step(&workflow);
    assert_eq!(step.id, "report", "the step id must be read");
    assert_eq!(
        step.env_value("SCCACHE_STATUS"),
        "${{ steps.setup-rust.outputs.sccache-status }}",
        "an expression value must be kept verbatim"
    );
    assert_eq!(
        step.env_value("RETRIES"),
        "3",
        "a number must render as text"
    );
    assert!(
        step.env.contains_key("EMPTY") && step.env_value("EMPTY").is_empty(),
        "an empty value must be present and empty"
    );
}

/// Absence is not an empty string: the map keeps the two apart even though
/// `env_value` reads both as `""`.
#[rstest]
fn an_absent_step_env_and_id_read_as_empty_and_absent() {
    let workflow = parse_with_step("      - run: y\n")
        .unwrap_or_else(|err| panic!("a step with no `env` must parse: {err}"));
    let step = only_step(&workflow);
    assert_eq!(step.id, "", "a step with no id must read as empty");
    assert!(
        step.env.is_empty(),
        "a step with no `env` must have no keys"
    );
    assert_eq!(step.env_value("ANY"), "", "an unset key must read as empty");
    assert!(
        !step.env.contains_key("ANY"),
        "an unset key must be absent from the map, not present and empty"
    );
}
