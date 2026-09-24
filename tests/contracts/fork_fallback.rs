//! Fork-fallback contracts: which lanes carry the arm, and how it is written.
//!
//! A pull request from a fork cannot obtain an Ubicloud runner, so the lane it
//! reaches names its runner through an expression that sends a fork to a
//! GitHub-hosted one. Split from `placement` because these ask a different
//! question: not what a lane costs, but whether a fork's pull request can
//! start it at all, and whether the declaration that decides that is well
//! formed.

use rstest::rstest;

use crate::workflow_assertions::{job_named, jobs, workflows};
use crate::workflow_estate::{Workflow, UBICLOUD_LABEL};

/// The runner a fork's pull request falls back to.
///
/// A fork cannot obtain an Ubicloud runner, and the point of the arm is that
/// it names one GitHub hosts, so this is the only value it may take.
const FORK_FALLBACK_LABEL: &str = "ubuntu-latest";

/// The context field that tells a fork's pull request from this repository's.
///
/// Named rather than matched by shape: `private` and `archived` sit in the
/// same position, parse the same way and evaluate, and either would send every
/// pull request down one arm.
const FORK_GUARD: &str = "github.event.pull_request.head.repo.fork";

/// A fork's pull request must be able to obtain the runner its lane names.
///
/// Without the arm the lane never starts on a fork's pull request, and a
/// required check that never reports presents as a pull request waiting rather
/// than as a placement fault.
#[rstest]
fn the_pull_request_lane_falls_back_to_a_hosted_runner_for_forks(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    assert_eq!(
        job.runs_on.guard(),
        Some(FORK_GUARD),
        "`build-test` must branch on `{FORK_GUARD}`; it declares {}",
        job.runs_on
    );
    assert_eq!(
        job.runs_on.fork_label(),
        Some(FORK_FALLBACK_LABEL),
        "a fork's pull request must be sent to `{FORK_FALLBACK_LABEL}`"
    );
    assert_eq!(
        job.runs_on.owned_label(),
        Some(UBICLOUD_LABEL),
        "this repository's own branches must keep `{UBICLOUD_LABEL}`"
    );
}

/// A lane no fork reaches must not carry an arm nothing takes.
///
/// `coverage-upload` runs on push, so the expression would add a branch to
/// keep correct for a case that cannot occur. Asserted so the arm does not
/// spread by imitation.
#[rstest]
fn lanes_no_fork_reaches_name_their_runner_outright(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "coverage-upload");
    assert_eq!(
        job.runs_on.labels(),
        [UBICLOUD_LABEL],
        "`coverage-upload` is on no pull-request lane, so it must name its \
         runner outright; it declares {}",
        job.runs_on
    );
}

/// A `runs-on` that parses to more than one line is a defect the run hides.
///
/// A folded scalar whose continuation is indented deeper than its key keeps
/// the break, so the expression arrives with a newline inside it. GitHub
/// evaluates the value regardless and the lane runs, so a green run is not
/// evidence that the declaration is well formed. This is the only place that
/// reads it.
#[rstest]
fn no_runs_on_declaration_carries_a_line_break(workflows: Vec<Workflow>) {
    let broken: Vec<String> = jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| {
            job.runs_on
                .labels()
                .iter()
                .any(|label| label.as_str().contains('\n'))
        })
        .map(|(file, job)| format!("{file}:{}", job.id))
        .collect();
    assert!(
        broken.is_empty(),
        "a `runs-on` must parse to one line; keep a folded scalar's \
         continuation at the same indent as its first line: {broken:?}"
    );
}
