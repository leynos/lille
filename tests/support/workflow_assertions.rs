//! Shared fixtures and assertion helpers for the workflow contracts.
//!
//! The contracts read the same estate and ask the same three questions of it:
//! give me that job, give me the step that uses that action, and tell me an
//! input matches. Keeping those here leaves each contract file holding only
//! the rules it asserts.
//!
//! # Examples
//!
//! ```no_run
//! let estate = workflow_assertions::workflows();
//! let job = workflow_assertions::job_named(&estate, "build-test");
//! assert_eq!(job.runs_on.labels(), ["ubicloud-standard-8"]);
//! ```

use rstest::fixture;

use crate::workflow_estate::Workflow;
use crate::workflow_loader::load_workflows;
use crate::workflow_model::{Job, Step};

/// Every workflow in `.github/workflows`, parsed once per test.
#[fixture]
pub fn workflows() -> Vec<Workflow> {
    match load_workflows() {
        Ok(estate) => estate,
        Err(err) => panic!("workflow estate must parse: {err}"),
    }
}

/// Returns every job in the estate, tagged with its workflow file.
pub fn jobs(workflows: &[Workflow]) -> Vec<(String, Job)> {
    workflows
        .iter()
        .flat_map(|workflow| {
            workflow
                .jobs
                .iter()
                .map(move |job| (workflow.file.clone(), job.clone()))
        })
        .collect()
}

/// Returns the job with the given id, or panics naming the missing job.
pub fn job_named<'a>(workflows: &'a [Workflow], id: &str) -> &'a Job {
    let found = workflows
        .iter()
        .flat_map(|workflow| workflow.jobs.iter())
        .find(|job| job.id == id);
    let Some(job) = found else {
        panic!("workflow estate must define the `{id}` job")
    };
    job
}

/// Returns a job's step that uses `coordinate`, or panics naming both.
///
/// `coordinate` is the whole action reference before the `@`, publisher
/// included, so a same-named action from another publisher cannot answer for
/// the one the contract meant.
pub fn step_using<'a>(job: &'a Job, coordinate: &str) -> &'a Step {
    let Some(step) = job.step_using(coordinate) else {
        panic!("`{}` must use the `{coordinate}` action", job.id)
    };
    step
}

/// Asserts that a step supplies the expected value for one input.
pub fn assert_input(job_id: &str, step: &Step, key: &str, expected: &str) {
    assert_eq!(
        step.input(key),
        expected,
        "`{job_id}` step `{}` must set `{key}: {expected}`",
        step.label()
    );
}
