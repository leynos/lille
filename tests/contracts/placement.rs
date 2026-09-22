//! Placement, cache-ownership, and job-shape contracts.
//!
//! Which runner a job uses, what it is allowed to bill, who owns each cached
//! path, and that the suite runs once rather than twice. These are the rules
//! that decide what the estate costs and whether a cache miss is explainable.

use std::collections::BTreeSet;

use rstest::rstest;

use crate::runner_selection::{is_github_hosted_label, RunnerLabel};
use crate::shared_action;
use crate::workflow_assertions::{assert_input, job_named, jobs, step_using, workflows};
use crate::workflow_cache_owners;
use crate::workflow_config::registered_runner_labels;
use crate::workflow_estate::{
    Workflow, WorkflowSource, BUILD_JOB_IDS, HOSTED_UBUNTU_LABEL, UBICLOUD_LABEL,
};
use crate::workflow_loader::{all_steps, parse_workflow};

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

/// Commands that would run the test suite a second time in a build job.
const REPEAT_TEST_COMMANDS: [&str; 4] = ["cargo test", "cargo nextest", "make test", "make all"];

/// Expression fragments the uv tool-layer cache key must carry.
const UV_CACHE_KEY_FRAGMENTS: [&str; 4] = [
    "runner.os",
    "runner.arch",
    "runner.environment",
    "hashFiles(",
];

#[rstest]
fn each_cached_path_has_exactly_one_owner(workflows: Vec<Workflow>) {
    let clashes: Vec<String> = jobs(&workflows)
        .into_iter()
        .flat_map(|(file, job)| {
            workflow_cache_owners::duplicated_paths(&job)
                .into_iter()
                .map(move |(path, owners)| format!("{file}:{}: {path} owned by {owners:?}", job.id))
        })
        .collect();
    assert!(
        clashes.is_empty(),
        "each cached path must have one owner: {clashes:?}"
    );
}

/// Every lane that is not a measured build sits on `ubuntu-latest`.
///
/// The label is compared exactly, because the guide names a label rather than
/// a family. `Job::stays_on_hosted_ubuntu` is the family question and stays
/// where it is; it answers who hosts the runner, and this answers which runner
/// the rule allows. Reading the family here would pass an API-bound job moved
/// to `ubuntu-24.04`, which is not wrong about hosting but has pinned an image
/// nobody decided to pin.
///
/// Over this repository's own workflows every reading agrees, so the
/// separation is driven case by case in `parsing.rs` and by the synthetic
/// estate below rather than by the tree.
/// Returns every non-build job that does not name the one allowed label.
fn misplaced_non_build_jobs(workflows: &[Workflow]) -> Vec<String> {
    jobs(workflows)
        .into_iter()
        .filter(|(_, job)| job.runs_on.names_a_runner())
        .filter(|(_, job)| !BUILD_JOB_IDS.contains(&job.id.as_str()))
        .filter(|(_, job)| job.runs_on.owned_label() != Some(HOSTED_UBUNTU_LABEL))
        .map(|(file, job)| format!("{file}:{}: {}", job.id, job.runs_on))
        .collect()
}

#[rstest]
fn non_build_jobs_stay_on_hosted_ubuntu_runners(workflows: Vec<Workflow>) {
    let misplaced = misplaced_non_build_jobs(&workflows);
    assert!(
        misplaced.is_empty(),
        "delayed-comment, metadata, and other API-bound jobs must name \
         `{HOSTED_UBUNTU_LABEL}` exactly: {misplaced:?}"
    );
}

/// A rule about a job is a statement about a job that runs.
///
/// Every placement, budget and cache rule in this module reads a job's
/// declared configuration. None of them reads whether the job executes, so
/// `if: false` on a build lane, or `continue-on-error: true`, leaves every one
/// of them satisfied while the lane runs nothing or reports success whatever
/// it found. The same holds one scope down, where a step whose `if` never
/// holds is a step the supply-chain and cache rules describe and the runner
/// never reaches.
///
/// Only a constant is refused. A condition that depends on the event is what
/// `if` is for, and `dependabot-automerge` declares one; the rule is that a
/// scope must be able to run, not that it must always run.
#[rstest]
fn no_scope_is_dead_or_advisory(workflows: Vec<Workflow>) {
    let dead: Vec<String> = jobs(&workflows)
        .into_iter()
        .flat_map(|(file, job)| {
            let mut offences = Vec::new();
            if job.never_runs() {
                offences.push(format!("{file}:{}: job `if` never holds", job.id));
            }
            if job.result_is_advisory() {
                offences.push(format!("{file}:{}: job failures do not count", job.id));
            }
            for step in &job.steps {
                if step.never_runs() {
                    offences.push(format!(
                        "{file}:{}: step `{}` has an `if` that never holds",
                        job.id,
                        step.label()
                    ));
                }
                if step.result_is_advisory() {
                    offences.push(format!(
                        "{file}:{}: step `{}` failures do not count",
                        job.id,
                        step.label()
                    ));
                }
            }
            offences
        })
        .collect();
    assert!(
        dead.is_empty(),
        "a scope that cannot run, or whose failure cannot fail the gate, \
         satisfies every rule written about it while enforcing none: {dead:?}"
    );
}

/// The measured bounds for each build job's `timeout-minutes`.
///
/// The lower bound keeps the timeout above the observed median so a normal run
/// cannot be killed; the upper bound keeps a hung run from billing for hours.
#[rstest]
#[case::build_test("build-test", 45, 120)]
#[case::coverage_upload("coverage-upload", 30, 90)]
fn build_jobs_keep_their_label_and_a_bounded_timeout(
    workflows: Vec<Workflow>,
    #[case] id: &str,
    #[case] lowest: u64,
    #[case] highest: u64,
) {
    let job = job_named(&workflows, id);
    assert_eq!(
        job.runs_on.owned_label(),
        Some(UBICLOUD_LABEL),
        "`{id}` must keep its measured runner label for this repository's own \
         branches; it declares {}",
        job.runs_on
    );
    let timeout = job
        .timeout_minutes
        .unwrap_or_else(|| panic!("`{id}` bills by the minute and must declare timeout-minutes"));
    assert!(
        (lowest..=highest).contains(&timeout),
        "`{id}` timeout-minutes {timeout} must lie between {lowest} and {highest}"
    );
}

/// A warm run has to be triggerable without pushing a commit, so the runner
/// and cache changes can be measured on an unchanged tree.
#[rstest]
fn the_pull_request_workflow_accepts_a_warm_run_dispatch(workflows: Vec<Workflow>) {
    let Some(ci) = workflows.iter().find(|workflow| workflow.file == "ci.yml") else {
        panic!("the estate must define ci.yml")
    };
    assert!(
        ci.has_trigger("workflow_dispatch"),
        "ci.yml must accept `workflow_dispatch` so a warm run can be measured \
         on demand; it declares {:?}",
        ci.triggers
    );
}

#[rstest]
fn every_runner_label_is_registered_with_actionlint(workflows: Vec<Workflow>) {
    let declared = registered_runner_labels()
        .unwrap_or_else(|err| panic!("actionlint configuration must be readable: {err}"));
    // Both arms of a conditional count as in use, minus the labels GitHub
    // hosts. A fork-fallback job carries one of each, and requiring its hosted
    // arm would put `ubuntu-latest` in the self-hosted registry.
    let in_use: BTreeSet<String> = jobs(&workflows)
        .into_iter()
        .flat_map(|(_, job)| job.runs_on.labels().to_vec())
        .map(|label: RunnerLabel| label.as_str().to_owned())
        .filter(|label| !is_github_hosted_label(label))
        .collect();
    let registered: BTreeSet<String> = declared.into_iter().collect();
    assert_eq!(
        registered, in_use,
        "`.github/actionlint.yaml` must register exactly the self-hosted \
         labels the workflows use. An unregistered label fails actionlint; a \
         registered but unused one hides a runner assignment that has already \
         been retired"
    );
}

#[rstest]
#[case::rust_toolchain("setup-rust", "cargo")]
#[case::whitaker_suite("install-whitaker", "whitaker ")]
fn an_installer_precedes_the_first_use_of_its_tool(
    workflows: Vec<Workflow>,
    #[case] installer: &str,
    #[case] first_use: &str,
) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let Some(use_index) = job.first_step_containing(first_use) else {
            continue;
        };
        let install_index = job
            .first_step_containing(installer)
            .unwrap_or_else(|| panic!("`{id}` uses `{first_use}` without a `{installer}` step"));
        assert!(
            install_index < use_index,
            "`{id}` must run `{installer}` before step {use_index} uses `{first_use}`"
        );
    }
}

#[rstest]
fn coverage_is_the_only_test_execution(workflows: Vec<Workflow>) {
    let duplicates: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, job, _)| BUILD_JOB_IDS.contains(&job.as_str()))
        .filter(|(_, _, step)| {
            REPEAT_TEST_COMMANDS
                .iter()
                .any(|command| step.run.contains(command))
        })
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.label()))
        .collect();
    assert!(
        duplicates.is_empty(),
        "the instrumented coverage run is the only test execution; drop the repeat: {duplicates:?}"
    );
}

#[rstest]
fn coverage_runs_the_whole_suite_once_and_owns_no_cargo_cache(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, &shared_action("generate-coverage"));
        for flag in ["all-features", "all-targets", "doctests"] {
            assert_input(id, step, flag, "true");
        }
        assert_input(id, step, "cache-provider", "external");
    }
}

#[rstest]
fn the_uv_cache_names_its_layers_and_keys_them_by_runner(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    let cache = job
        .steps
        .iter()
        .find(|step| step.cache_paths().iter().any(|path| path == ".uv-cache"));
    let Some(step) = cache else {
        panic!("`build-test` must cache the uv download layer")
    };
    assert_eq!(
        step.cache_paths(),
        vec![".uv-cache".to_owned(), ".uv-tools".to_owned()],
        "the uv cache must own both the download store and the tool store"
    );
    let key = step.input("key");
    for fragment in UV_CACHE_KEY_FRAGMENTS {
        assert!(
            key.contains(fragment),
            "the uv cache key `{key}` must vary with `{fragment}`"
        );
    }
}

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

/// The exact label and the family are different questions, and the rule is the
/// label.
///
/// This repository has no API-bound job on a pinned Ubuntu image, so over its
/// own workflows the contract above passes whichever reading it uses. The
/// separation is driven here: `ubuntu-24.04` is GitHub-hosted, is Ubuntu, and
/// is still not what the rule allows.
#[rstest]
#[case::the_allowed_label("ubuntu-latest", true)]
#[case::a_pinned_image_of_the_same_family("ubuntu-24.04", false)]
#[case::another_pinned_image("ubuntu-22.04", false)]
#[case::a_hosted_runner_of_another_platform("windows-latest", false)]
#[case::the_paid_label("ubicloud-standard-4", false)]
fn only_one_hosted_label_is_allowed_for_a_non_build_job(
    #[case] label: &str,
    #[case] allowed: bool,
) {
    let text = format!("on: push\njobs:\n  a:\n    runs-on: {label}\n    steps: []\n");
    let workflow = match parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text: &text,
    }) {
        Ok(parsed) => parsed,
        Err(err) => panic!("`{label}` must parse: {err}"),
    };
    let Some(job) = workflow.jobs.first() else {
        panic!("`{label}` must yield a job")
    };
    assert_eq!(
        job.runs_on.owned_label() == Some(HOSTED_UBUNTU_LABEL),
        allowed,
        "`{label}` must read as allowed={allowed} for a job that is not a build"
    );
    assert!(
        job.stays_on_hosted_ubuntu() == label.starts_with("ubuntu-"),
        "`{label}` must still answer the family question by family"
    );
}

/// The rule itself, over an estate built to separate the two readings.
///
/// The case below drives `owned_label` directly, which says nothing about how
/// the rule reads it: swapping the rule back to the family predicate left that
/// case green. This drives the rule, so the swap fails here.
#[rstest]
#[case::the_allowed_label("ubuntu-latest", 0)]
#[case::a_pinned_image_of_the_same_family("ubuntu-24.04", 1)]
#[case::a_hosted_runner_of_another_platform("windows-latest", 1)]
fn the_rule_reports_a_non_build_job_on_any_other_label(
    #[case] label: &str,
    #[case] expected: usize,
) {
    let text =
        format!("on: push\njobs:\n  delayed-comment:\n    runs-on: {label}\n    steps: []\n");
    let workflow = match parse_workflow(WorkflowSource {
        file: "scratch.yml",
        text: &text,
    }) {
        Ok(parsed) => parsed,
        Err(err) => panic!("`{label}` must parse: {err}"),
    };
    let misplaced = misplaced_non_build_jobs(&[workflow]);
    assert_eq!(
        misplaced.len(),
        expected,
        "a non-build job on `{label}` must yield {expected} offence(s); the \
         rule gave {misplaced:?}"
    );
}
