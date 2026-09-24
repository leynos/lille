//! Which workflows a pull request can cause to run, and what they forward.
//!
//! A pull request reaches a workflow in two ways: by a trigger the workflow
//! declares, and through a local reusable-workflow call made by a workflow it
//! already reaches. The second is the one a trigger-based reading misses,
//! because a reusable child declares `workflow_call`, not `pull_request`, and
//! so reads as unreachable while a pull request runs it with whatever secrets
//! its caller forwarded.
//!
//! The readers take parsed workflows and, where the typed model does not carry
//! the field, the raw document, so the contracts can drive graphs and job
//! shapes this repository does not have.
//!
//! # Examples
//!
//! ```no_run
//! assert_eq!(
//!     coverage_reach::local_workflow_target("./.github/workflows/child.yml"),
//!     Some("child.yml")
//! );
//! ```

use std::collections::VecDeque;

use serde_norway::Value;

use crate::workflow_estate::Workflow;

/// Where a same-repository reusable workflow lives, relative to the root.
pub const WORKFLOW_DIRECTORY: &str = ".github/workflows/";

/// The spellings GitHub accepts for a call into this repository.
const LOCAL_CALL_PREFIXES: [&str; 2] = ["./", "$/"];

/// The `secrets:` value that forwards every secret the caller holds.
///
/// The credential goes with it, and the caller's text never names it, so
/// neither the raw nor the parsed credential sweep can see the forwarding.
pub const INHERITED_SECRETS: &str = "inherit";

/// Returns the workflow file a job-level `uses` names in this repository.
///
/// A call is local by its shape: with one leading `./` or `$/` removed, the
/// remainder is a path under the workflow directory. `$/` is GitHub's
/// self-repository form, resolved at the running commit, so `$/...@ref` names
/// a file that does not exist and the walk reports it as missing. Anything else is a call into another repository and is not
/// followed, since a foreign workflow is not ours to read.
///
/// ```no_run
/// assert_eq!(coverage_reach::local_workflow_target("o/r/.github/workflows/w.yml@v1"), None);
/// ```
#[must_use]
pub fn local_workflow_target(uses: &str) -> Option<&str> {
    let path = LOCAL_CALL_PREFIXES
        .iter()
        .find_map(|prefix| uses.strip_prefix(prefix))
        .unwrap_or(uses);
    path.strip_prefix(WORKFLOW_DIRECTORY)
}

/// The workflows one entry reaches, and the local calls that name nothing.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reach {
    /// The entry first, then each local workflow it reaches, each once.
    pub reached: Vec<String>,
    /// Local calls naming a workflow the estate does not hold.
    pub missing: Vec<String>,
}

/// Returns the entry and every local workflow it reaches, transitively.
///
/// The walk tracks what it has seen, so a cycle, which GitHub rejects but a
/// half-finished edit can produce, terminates rather than recursing. A local
/// call to a file the estate does not hold is returned as missing rather than
/// skipped: a skipped target is a workflow no rule was ever asked about.
#[must_use]
pub fn reachable_workflows(entry: &str, workflows: &[Workflow]) -> Reach {
    let mut reach = Reach::default();
    let mut pending = VecDeque::from([entry.to_owned()]);
    while let Some(current) = pending.pop_front() {
        if reach.reached.contains(&current) || reach.missing.contains(&current) {
            continue;
        }
        let Some(workflow) = workflows.iter().find(|workflow| workflow.file == current) else {
            reach.missing.push(current);
            continue;
        };
        pending.extend(
            workflow
                .jobs
                .iter()
                .filter_map(|job| local_workflow_target(&job.uses))
                .map(ToOwned::to_owned),
        );
        reach.reached.push(current);
    }
    reach
}

/// Returns the id of every job in a raw document that passes `secrets: inherit`.
///
/// The typed model does not carry `secrets`, because no other rule asks about
/// it, so this reads the document the model was parsed from.
///
/// # Errors
///
/// Returns the parser's error when the text is not YAML. The loader refuses
/// such a document, so reaching this means the raw text and the parsed
/// workflow have come apart, and answering "no job" would hide exactly that.
pub fn jobs_inheriting_secrets(raw_text: &str) -> Result<Vec<String>, serde_norway::Error> {
    let document: Value = serde_norway::from_str(raw_text)?;
    let Some(jobs) = document.get("jobs").and_then(Value::as_mapping) else {
        return Ok(Vec::new());
    };
    Ok(jobs
        .iter()
        .filter(|(_, job)| job.get("secrets").and_then(Value::as_str) == Some(INHERITED_SECRETS))
        .filter_map(|(id, _)| id.as_str().map(ToOwned::to_owned))
        .collect())
}
