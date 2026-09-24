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

/// The merge queue's trigger.
///
/// It runs the pull request's merged code with the repository's secrets and
/// gates the merge, so a `CodeScene` failure there blocks the queue exactly as
/// it once blocked the pull request.
pub const MERGE_QUEUE_TRIGGER: &str = "merge_group";

/// The triggers a review or a review comment on a pull request fires.
pub const REVIEW_TRIGGERS: [&str; 2] = ["pull_request_review", "pull_request_review_comment"];

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

/// One including line of an artefact step's `path` input.
struct ArtefactPathEntry<'a>(&'a str);

impl ArtefactPathEntry<'_> {
    /// Reports whether the entry can select the report.
    ///
    /// The report sits at the workspace root, so an entry reaches it when it
    /// names the report, or when its first segment is the root, its parent, or
    /// anything the reader cannot resolve: a glob, a brace set, or an
    /// expression such as `${{ github.workspace }}`. Reading those as
    /// publication errs towards the loud failure: a narrow upload spelt with a
    /// leading glob fails the contract, where the other reading lets `path: .`
    /// publish the report unseen.
    fn reaches_the_report(&self) -> bool {
        let mut rest = self.0;
        while let Some(stripped) = rest.strip_prefix("./") {
            rest = stripped;
        }
        let first = rest.split('/').next().unwrap_or_default();
        rest.contains(COVERAGE_REPORT_PATH)
            || matches!(first, "" | "." | "..")
            || first.contains(['*', '?', '[', '{', '$'])
    }
}

/// Reports whether a step publishes the coverage report as an artefact.
///
/// The path input holds one pattern per line, and a line opening with `!`
/// only excludes, so the step publishes when any including line reaches the
/// report. A step that names no path uploads the workspace, which holds the
/// generated report, so it fails closed rather than reading as an exemption.
///
/// ```no_run
/// let mut step = workflow_model::Step::default();
/// step.uses = "actions/upload-artifact@v4".to_owned();
/// step.with.insert("path".to_owned(), "./**".to_owned());
/// assert!(coverage_boundary::publishes_the_coverage_report(&step));
/// ```
#[must_use]
pub fn publishes_the_coverage_report(step: &Step) -> bool {
    if action_of(step) != PUBLISH_ARTEFACT_ACTION {
        return false;
    }
    step.with.get("path").is_none_or(|path| {
        path.lines()
            .map(str::trim)
            .filter(|entry| !entry.is_empty() && !entry.starts_with('!'))
            .map(ArtefactPathEntry)
            .any(|entry| entry.reaches_the_report())
    })
}

/// Reports whether a step tells the coverage action not to archive.
#[must_use]
pub fn declines_the_generated_report_archive(step: &Step) -> bool {
    action_of(step) == GENERATE_COVERAGE_ACTION
        && step.input(PUBLICATION_OPT_OUT_INPUT) == PUBLICATION_OPT_OUT_VALUE
}

/// Reports whether a workflow can be reached by a pull request.
///
/// Every trigger a pull request's activity fires counts. `pull_request_target`
/// and `workflow_run` resume in the base repository's context, so a coverage
/// step under either is a credential a pull request's contents can influence;
/// the merge queue runs the merged code with secrets and gates the merge; and
/// a review or review comment starts a run as surely as a push to the branch.
#[must_use]
pub fn is_reachable_by_a_pull_request(workflow: &Workflow) -> bool {
    [
        PULL_REQUEST_TRIGGER,
        PULL_REQUEST_TARGET_TRIGGER,
        SUBMISSION_TRIGGER,
        MERGE_QUEUE_TRIGGER,
    ]
    .iter()
    .chain(REVIEW_TRIGGERS.iter())
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

/// The names a pull-request workflow's raw text may not contain at all.
const RAW_TEXT_PROHIBITIONS: [&str; 2] = [CREDENTIAL_ENVIRONMENT_KEY, CODESCENE_HOST];

/// Returns one offence for each prohibited name the raw text holds, ignoring
/// case.
///
/// GitHub resolves `secrets.cs_access_token` to the same secret as the
/// upper-case spelling, and a host name is case-insensitive, so a
/// case-sensitive search is one keystroke from blind.
fn raw_text_offences(name: &str, raw_text: &str) -> Vec<String> {
    let folded = raw_text.to_lowercase();
    RAW_TEXT_PROHIBITIONS
        .iter()
        .filter(|needle| folded.contains(&needle.to_lowercase()))
        .map(|needle| format!("{name}: raw text references {needle}"))
        .collect()
}

/// Returns every prohibited coverage-surface reference in one workflow.
///
/// The raw text is taken alongside the parsed document because the credential
/// and the host must not be present at all: a reference inside a comment, or
/// in a shape the parser did not keep, is still a reference. It is also where
/// `secrets: inherit` is read, since the typed model does not carry `secrets`.
///
/// Raw text that does not parse is reported as an offence of its own rather
/// than read as forwarding nothing: the product here is the list the contract
/// asserts empty, so the failure reaches the contract boundary by name.
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
    match jobs_inheriting_secrets(raw_text) {
        Ok(jobs) => offenders.extend(jobs.into_iter().map(|job| {
            format!(
                "{name}:{job} forwards every secret (secrets: {INHERITED_SECRETS}), the \
                 credential included"
            )
        })),
        Err(err) => offenders.push(format!(
            "{name}: raw text does not parse, so `secrets: {INHERITED_SECRETS}` could not be \
             read: {err}"
        )),
    }
    offenders.extend(raw_text_offences(name, raw_text));
    offenders
}

/// Returns every offence in a workflow and in each local workflow it reaches.
///
/// Every pull-request clause runs over the closure rather than the entry,
/// because a reusable child declares `workflow_call` alone and so reads as
/// unreachable while the pull request runs it with the caller's secrets. A
/// local call naming a workflow that is not there is an offence of its own, and
/// so is a reached workflow with no raw text: its raw-text rules could not run.
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
            raw_texts.get(&workflow.file).map_or_else(
                || {
                    vec![format!(
                        "{}: no raw text was supplied, so its raw-text rules could not run",
                        workflow.file
                    )]
                },
                |raw| coverage_surface_offenders(workflow, raw),
            )
        })
        .collect();
    offenders.extend(
        reach.missing.iter().map(|missing| {
            format!("{entry} reaches a local workflow {missing} no rule could read")
        }),
    );
    offenders
}
