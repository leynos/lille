//! What a pull-request-reachable workflow may not touch.
//!
//! Pull-request CI generates `lcov.info` and compares it with the ratcheted
//! baseline derived from `main`. It does not publish that report as an
//! artefact, invoke the `CodeScene` coverage action, name the `CodeScene` host,
//! forward every secret with `secrets: inherit`, or carry the `CodeScene`
//! credential. Those belong to `coverage-main.yml`, which is the only writer
//! of persistent coverage state. The rules run over every local workflow a
//! pull-request lane calls, not over the lane alone.
//!
//! The coverage action archives the report it generated under a step of its
//! own, so declining that archive is part of the same boundary: a caller that
//! reaches the action without the opt-out has published the report whether or
//! not the workflow declares an artefact step. That rule is checked here
//! rather than in the workflow, because the action's own step is not the
//! caller's to see.
//!
//! These readers take a parsed workflow and its raw text rather than reading
//! files, so the contract can drive them over shapes this repository does not
//! have. Parameterized over the repository's own workflows alone, a reader
//! that answered nothing would agree with a correct one exactly.
//!
//! # Examples
//!
//! ```no_run
//! let step = workflow_model::Step::default();
//! assert!(!coverage_boundary::publishes_the_coverage_report(&step));
//! ```

use std::collections::BTreeMap;

use crate::coverage_reach::{jobs_inheriting_secrets, reachable_workflows, INHERITED_SECRETS};
use crate::workflow_estate::Workflow;
use crate::workflow_model::Step;

/// The action that generates coverage.
///
/// A pull request calls it in ratchet mode and stops there; `main` calls it to
/// produce the report it publishes.
pub const GENERATE_COVERAGE_ACTION: &str =
    "leynos/shared-actions/.github/actions/generate-coverage";

/// The action that submits a report to `CodeScene`, in either of its modes.
///
/// `main` owns this call. A pull-request lane that makes it turns a coverage
/// comparison into a step that needs a credential, which is what CV-005
/// removes.
pub const UPLOAD_COVERAGE_ACTION: &str =
    "leynos/shared-actions/.github/actions/upload-codescene-coverage";

/// The generic artefact action.
///
/// A pull request must not carry the report to it under any step name.
pub const PUBLISH_ARTEFACT_ACTION: &str = "actions/upload-artifact";

/// The input that suppresses the coverage action's own archive step.
pub const PUBLICATION_OPT_OUT_INPUT: &str = "publish-artefact";

/// The value that suppresses it, compared as the string the action compares.
pub const PUBLICATION_OPT_OUT_VALUE: &str = "false";

/// The credential the `CodeScene` upload reads.
///
/// It must not appear in a workflow a pull request can reach, in a parsed
/// value or anywhere in the raw text. The raw reading is what catches a
/// reference in a comment or in a shape the parser flattened away.
pub const CREDENTIAL_ENVIRONMENT_KEY: &str = "CS_ACCESS_TOKEN";

/// The command form of the same upload, which needs no action reference.
pub const COVERAGE_COMMAND: &str = "cs-coverage";

/// The report the coverage action writes, and the one `CodeScene` is sent.
pub const COVERAGE_REPORT_PATH: &str = "lcov.info";

/// The service itself.
///
/// A pull-request lane that names its host is talking to it by some route
/// other than the action, which is the same dependency the boundary exists to
/// remove, spelt as a `curl`.
pub const CODESCENE_HOST: &str = "codescene.io";

/// The trigger a fork's pull request fires, which cannot read secrets.
pub const PULL_REQUEST_TRIGGER: &str = "pull_request";

/// The variant that runs in the base repository's context.
///
/// It *can* read the repository's secrets, unlike `pull_request`, so a
/// coverage step here would be worse than one in an ordinary pull-request job
/// rather than equivalent to it.
pub const PULL_REQUEST_TARGET_TRIGGER: &str = "pull_request_target";

/// The trigger that resumes a run with the base repository's privileges.
pub const SUBMISSION_TRIGGER: &str = "workflow_run";

/// Returns a step's action reference without its version.
///
/// Splitting on the version separator rather than matching a prefix keeps
/// `upload-codescene-coverage-legacy` from reading as the real action.
///
/// ```no_run
/// let mut step = workflow_model::Step::default();
/// step.uses = "actions/checkout@v4".to_owned();
/// assert_eq!(coverage_boundary::action_of(&step), "actions/checkout");
/// ```
#[must_use]
pub fn action_of(step: &Step) -> &str {
    step.uses
        .split_once('@')
        .map_or(step.uses.as_str(), |(name, _)| name)
}

/// Reports whether a step publishes the coverage report as an artefact.
///
/// A step of the artefact action that names no path uploads the workspace,
/// which holds the generated report, so it fails closed rather than reading as
/// an exemption.
#[must_use]
pub fn publishes_the_coverage_report(step: &Step) -> bool {
    if action_of(step) != PUBLISH_ARTEFACT_ACTION {
        return false;
    }
    step.with
        .get("path")
        .is_none_or(|path| path.contains(COVERAGE_REPORT_PATH))
}

/// Reports whether a step tells the coverage action not to archive.
#[must_use]
pub fn declines_the_generated_report_archive(step: &Step) -> bool {
    action_of(step) == GENERATE_COVERAGE_ACTION
        && step.input(PUBLICATION_OPT_OUT_INPUT) == PUBLICATION_OPT_OUT_VALUE
}

/// Reports whether a workflow can be reached by a pull request.
///
/// All three triggers count. `pull_request_target` and `workflow_run` resume
/// in the base repository's context, so a coverage step under either is a
/// credential a pull request's contents can influence.
#[must_use]
pub fn is_reachable_by_a_pull_request(workflow: &Workflow) -> bool {
    [
        PULL_REQUEST_TRIGGER,
        PULL_REQUEST_TARGET_TRIGGER,
        SUBMISSION_TRIGGER,
    ]
    .iter()
    .any(|trigger| workflow.has_trigger(trigger))
}

/// Returns every prohibited reference one step makes, naming where it is.
///
/// Split from the sweep below so the sweep is two loops and a reduction rather
/// than two loops wrapped around four rules: the rules are what change, and
/// they change one at a time.
fn step_offences(where_: &str, step: &Step) -> Vec<String> {
    let mut offences = Vec::new();
    if publishes_the_coverage_report(step) {
        offences.push(format!(
            "{where_} publishes the coverage report as an artefact"
        ));
    }
    if action_of(step) == GENERATE_COVERAGE_ACTION && !declines_the_generated_report_archive(step) {
        offences.push(format!(
            "{where_} invokes the coverage action without declining its own \
             archive ({PUBLICATION_OPT_OUT_INPUT}: {PUBLICATION_OPT_OUT_VALUE})"
        ));
    }
    if action_of(step) == UPLOAD_COVERAGE_ACTION {
        offences.push(format!("{where_} invokes the CodeScene coverage action"));
    }
    if step.run.contains(COVERAGE_COMMAND) {
        offences.push(format!("{where_} runs a {COVERAGE_COMMAND} command"));
    }
    offences
}

/// Returns one offence when the raw text names `needle`, ignoring case.
///
/// GitHub resolves `secrets.cs_access_token` to the same secret as the
/// upper-case spelling, and a host name is case-insensitive, so a
/// case-sensitive search is one keystroke from blind.
fn mentions(name: &str, raw_text: &str, needle: &str) -> Option<String> {
    raw_text
        .to_lowercase()
        .contains(&needle.to_lowercase())
        .then(|| format!("{name}: raw text references {needle}"))
}

/// Returns every prohibited coverage-surface reference in one workflow.
///
/// The raw text is taken alongside the parsed document because the credential
/// and the host must not be present at all: a reference inside a comment, or
/// in a shape the parser did not keep, is still a reference. It is also where
/// `secrets: inherit` is read, since the typed model does not carry `secrets`.
#[must_use]
pub fn coverage_surface_offenders(workflow: &Workflow, raw_text: &str) -> Vec<String> {
    let name = &workflow.file;
    let mut offenders: Vec<String> = workflow
        .jobs
        .iter()
        .flat_map(|job| {
            job.steps.iter().enumerate().flat_map(move |(index, step)| {
                step_offences(&format!("{name}:{}: step {index}", job.id), step)
            })
        })
        .collect();
    offenders.extend(jobs_inheriting_secrets(raw_text).into_iter().map(|job| {
        format!(
            "{name}:{job} forwards every secret (secrets: {INHERITED_SECRETS}), the \
             credential included"
        )
    }));
    offenders.extend(mentions(name, raw_text, CREDENTIAL_ENVIRONMENT_KEY));
    offenders.extend(mentions(name, raw_text, CODESCENE_HOST));
    offenders
}

/// Returns every offence in a workflow and in each local workflow it reaches.
///
/// Every pull-request clause runs over the closure rather than the entry,
/// because a reusable child declares `workflow_call` alone and so reads as
/// unreachable while the pull request runs it with the caller's secrets. A
/// local call naming a workflow that is not there is an offence of its own.
#[must_use]
pub fn pull_request_offenders(
    entry: &str,
    workflows: &[Workflow],
    raw_texts: &BTreeMap<String, String>,
) -> Vec<String> {
    let reach = reachable_workflows(entry, workflows);
    let mut offenders: Vec<String> = reach
        .reached
        .iter()
        .filter_map(|file| workflows.iter().find(|workflow| &workflow.file == file))
        .flat_map(|workflow| {
            let raw = raw_texts.get(&workflow.file).map_or("", String::as_str);
            coverage_surface_offenders(workflow, raw)
        })
        .collect();
    offenders.extend(
        reach.missing.iter().map(|missing| {
            format!("{entry} reaches a local workflow {missing} no rule could read")
        }),
    );
    offenders
}
