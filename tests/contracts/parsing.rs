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
use crate::workflow_model::{is_github_hosted_label, is_hosted_label, RunnerSelection};

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

/// The expression reader must classify only the shape it exists to read.
///
/// A literal label, a matrix reference and an expression of another shape must
/// all read as no fork fallback, or a job the reader misclassified would be
/// held to the wrong rules while every assertion still passed. The line-break
/// case is the one that matters most: a declaration carrying a newline must
/// read as no fork fallback here, so the assertion written for it reports it
/// rather than this reader quietly parsing through it.
#[rstest]
#[case::the_deployed_shape(
    "${{ github.event.pull_request.head.repo.fork && 'ubuntu-latest' || 'ubicloud-standard-4' }}",
    Some(("github.event.pull_request.head.repo.fork", "ubuntu-latest", "ubicloud-standard-4"))
)]
#[case::generous_internal_spacing(
    "${{   github.event.pull_request.head.repo.fork   &&   'a'   ||   'b'   }}",
    Some(("github.event.pull_request.head.repo.fork", "a", "b"))
)]
#[case::a_literal_label("ubuntu-latest", None)]
#[case::a_matrix_reference("${{ matrix.os }}", None)]
#[case::a_single_armed_expression("${{ github.event.pull_request.head.repo.fork && 'a' }}", None)]
// The estate prescribes one spelling. A negated guard or a comparison says the
// same thing with the arms the other way round, and reading either as the
// prescribed form would let two spellings of the placement drift apart while
// both satisfied the contract. They read as no fork fallback, so the lane is
// reported as not declaring it.
#[case::a_negated_guard("${{ !github.event.pull_request.head.repo.fork && 'a' || 'b' }}", None)]
#[case::a_compared_guard(
    "${{ github.event.pull_request.head.repo.owner == 'leynos' && 'a' || 'b' }}",
    None
)]
#[case::a_declaration_carrying_a_line_break(
    "${{ github.event.pull_request.head.repo.fork\n&& 'a' || 'b' }}",
    None
)]
fn the_expression_reader_accepts_one_shape_and_refuses_the_rest(
    #[case] text: &str,
    #[case] expected: Option<(&str, &str, &str)>,
) {
    let read = RunnerSelection::from_expression(text);
    let rendered = read.as_ref().map(|selection| {
        (
            selection.guard().unwrap_or_default(),
            selection.fork_label().unwrap_or_default(),
            selection.owned_label().unwrap_or_default(),
        )
    });
    assert_eq!(rendered, expected, "`{text}` was read as {read:?}");
}

/// A label is GitHub-hosted by its image prefix, whatever the job around it is.
///
/// Read per label because a fork-fallback selection holds one hosted and one
/// self-hosted, and the actionlint registry must be asked only about the
/// second.
#[rstest]
#[case::ubuntu("ubuntu-latest", true)]
#[case::windows("windows-2022", true)]
#[case::macos("macos-14", true)]
#[case::ubicloud("ubicloud-standard-4", false)]
#[case::self_hosted("self-hosted", false)]
#[case::a_prefix_without_its_separator("ubuntulatest", false)]
fn a_hosted_label_is_told_from_a_self_hosted_one(#[case] label: &str, #[case] hosted: bool) {
    assert_eq!(
        is_hosted_label(label),
        hosted,
        "`{label}` was misclassified"
    );
}

/// The registry question asks by name, not by prefix, and the two differ.
///
/// A prefix test absorbs any new label that looks hosted, so a lane moved onto
/// an unknown image would drop out of "in use" and its registration would go
/// unnoticed. `ubuntu-20.04` is the case that separates them: a hosted family,
/// a label this estate does not use, and one that must therefore be reported
/// rather than silently excused.
#[rstest]
#[case::a_named_hosted_label("ubuntu-latest", true)]
#[case::another_named_one("macos-latest", true)]
#[case::a_hosted_family_member_not_named("ubuntu-20.04", false)]
#[case::the_paid_label("ubicloud-standard-4", false)]
fn the_registry_reads_hosted_labels_by_name(#[case] label: &str, #[case] hosted: bool) {
    assert_eq!(
        is_github_hosted_label(label),
        hosted,
        "`{label}` must be classified by name for the registry question"
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
