//! Whether a declared scope can run, and whether its result can fail.
//!
//! Split from `parsing`, which is about turning a workflow file into the
//! model. These cases are about two fields in that model, `if` and
//! `continue-on-error`, and what the rest of the suite is entitled to assume
//! when it reads a job or a step.
//!
//! The rules that consume them live in `placement`. They are driven here
//! because this repository's workflows carry no dead scope, so the rules pass
//! over the tree whether the readers work or not.

use rstest::rstest;

use crate::workflow_estate::WorkflowSource;
use crate::workflow_loader::parse_workflow;
use crate::workflow_model::{is_constantly_false, is_constantly_true};

/// A constant condition is read from the value, not from its spelling.
///
/// A workflow may write `if` and `continue-on-error` as bare YAML booleans or
/// as expressions, and the loader renders both to a string. Recognising only
/// the bare spelling would let `if: ${{ false }}` describe a dead job that
/// every rule went on passing.
///
/// A condition naming a context is not constant and is not refused: that is
/// what `if` is for, and `dependabot-automerge` declares one.
#[rstest]
#[case::bare_false("false", true, false)]
#[case::bare_true("true", false, true)]
#[case::an_expression_false("${{ false }}", true, false)]
#[case::an_expression_false_unspaced("${{false}}", true, false)]
#[case::an_expression_true("${{ true }}", false, true)]
#[case::padded("  false  ", true, false)]
#[case::a_context_condition("github.event_name == 'pull_request'", false, false)]
#[case::always("always()", false, false)]
// An empty condition is falsy to GitHub, so the scope never runs. An
// *absent* condition is a different thing and the model keeps the two
// apart; only a present, empty one reaches this reader.
#[case::empty("", true, false)]
fn a_constant_condition_is_told_from_one_that_depends_on_the_event(
    #[case] condition: &str,
    #[case] never: bool,
    #[case] always: bool,
) {
    assert_eq!(
        is_constantly_false(condition),
        never,
        "`{condition}` was misread as a condition that never holds"
    );
    assert_eq!(
        is_constantly_true(condition),
        always,
        "`{condition}` was misread as a condition that always holds"
    );
}

/// What one scope should read as: whether it runs, and whether its result
/// counts.
///
/// The same pair of questions is asked of a job and of a step, so it is one
/// type used twice rather than four fields side by side. Four booleans in a
/// row read as `(false, false, true, false)` at a call site, where a
/// transposed pair says nothing and would survive review.
#[derive(Clone, Copy)]
struct Scope {
    never_runs: bool,
    advisory: bool,
}

impl Scope {
    /// Runs, and its failure fails the workflow: the ordinary case.
    const LIVE: Self = Self {
        never_runs: false,
        advisory: false,
    };
    /// Declared, and cannot run.
    const DEAD: Self = Self {
        never_runs: true,
        advisory: false,
    };
    /// Runs, and its failure is reported as success.
    const ADVISORY: Self = Self {
        never_runs: false,
        advisory: true,
    };
}

/// The two scopes of a fixture workflow's single job and single step.
#[derive(Clone, Copy)]
struct Scopes {
    job: Scope,
    step: Scope,
}

impl Scopes {
    /// Both scopes live and mandatory.
    const LIVE: Self = Self {
        job: Scope::LIVE,
        step: Scope::LIVE,
    };
}

/// A dead or advisory scope is reported, and an absent field is neither.
///
/// The contract that reads these lives in `placement.rs` and runs over this
/// repository's own workflows, none of which carries a dead scope, so it would
/// pass with the readers deleted. The readers are driven here instead.
#[rstest]
#[case::no_declarations(
    "on: push\njobs:\n  a:\n    runs-on: x\n    steps:\n      - run: echo\n",
    Scopes::LIVE
)]
#[case::a_dead_job(
    "on: push\njobs:\n  a:\n    runs-on: x\n    if: false\n    steps:\n      - run: echo\n",
    Scopes { job: Scope::DEAD, ..Scopes::LIVE }
)]
#[case::an_advisory_job(
    "on: push\njobs:\n  a:\n    runs-on: x\n    continue-on-error: true\n    steps:\n      - run: echo\n",
    Scopes { job: Scope::ADVISORY, ..Scopes::LIVE }
)]
#[case::a_dead_step(
    "on: push\njobs:\n  a:\n    runs-on: x\n    steps:\n      - run: echo\n        if: ${{ false }}\n",
    Scopes { step: Scope::DEAD, ..Scopes::LIVE }
)]
#[case::an_advisory_step(
    "on: push\njobs:\n  a:\n    runs-on: x\n    steps:\n      - run: echo\n        continue-on-error: true\n",
    Scopes { step: Scope::ADVISORY, ..Scopes::LIVE }
)]
#[case::a_live_condition(
    "on: push\njobs:\n  a:\n    runs-on: x\n    if: github.event_name == 'push'\n    steps:\n      - run: echo\n        if: always()\n",
    Scopes::LIVE
)]
fn a_dead_or_advisory_scope_is_reported_at_either_level(
    #[case] text: &str,
    #[case] expected: Scopes,
) {
    let workflow = parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text,
    })
    .expect("the fixture workflow must parse");
    let job = workflow.jobs.first().expect("the fixture declares one job");
    let step = job.steps.first().expect("the fixture declares one step");
    assert_eq!(
        job.never_runs(),
        expected.job.never_runs,
        "job condition misread"
    );
    assert_eq!(
        job.result_is_advisory(),
        expected.job.advisory,
        "job `continue-on-error` misread"
    );
    assert_eq!(
        step.never_runs(),
        expected.step.never_runs,
        "step condition misread"
    );
    assert_eq!(
        step.result_is_advisory(),
        expected.step.advisory,
        "step `continue-on-error` misread"
    );
}
