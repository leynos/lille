//! Structural contracts over the repository's GitHub Actions workflows.
//!
//! These tests encode the Ubicloud adoption rules that a reviewer would
//! otherwise have to re-check by hand on every workflow edit: no tool is built
//! from source, every cached path has exactly one owner, cache and shared
//! action references are pinned, API-bound jobs stay GitHub-hosted, and an
//! installer always precedes the first use of what it installs. They also pin
//! the inputs that make those rules true, so a workflow cannot keep the shape
//! of the policy while dropping its substance.
//!
//! They read the workflow files directly, so they fail on the change that
//! introduces a violation rather than on the CI run that suffers from it.

#[path = "support/workflow_cache_owners.rs"]
mod workflow_cache_owners;
#[path = "support/workflow_model.rs"]
mod workflow_model;

use camino::Utf8Path;
use rstest::{fixture, rstest};

use workflow_model::{
    all_steps, load_workflows, load_workflows_in, parse_workflow, read_repository_file, Job, Step,
    Workflow, BUILD_JOB_IDS, CACHE_ACTION_SHA, SHARED_ACTIONS_SHA, UBICLOUD_LABEL,
};

/// Fragments that mark a step as building a tool from source.
///
/// `cargo binstall` is included because it compiles whenever its default
/// strategies fall through to `compile`; the estate's rule is to install from
/// a verified release archive instead.
const SOURCE_BUILD_FRAGMENTS: [&str; 3] = ["cargo install", "cargo-binstall ", "cargo binstall"];

/// Commands that would run the test suite a second time in a build job.
const REPEAT_TEST_COMMANDS: [&str; 4] = ["cargo test", "cargo nextest", "make test", "make all"];

/// Expression fragments the uv tool-layer cache key must carry.
const UV_CACHE_KEY_FRAGMENTS: [&str; 4] = [
    "runner.os",
    "runner.arch",
    "runner.environment",
    "hashFiles(",
];

/// Every workflow in `.github/workflows`, parsed once per test.
#[fixture]
fn workflows() -> Vec<Workflow> {
    load_workflows().unwrap_or_else(|err| panic!("workflow estate must parse: {err}"))
}

/// Returns every job in the estate, tagged with its workflow file.
fn jobs(workflows: &[Workflow]) -> Vec<(String, Job)> {
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
fn job_named<'a>(workflows: &'a [Workflow], id: &str) -> &'a Job {
    workflows
        .iter()
        .flat_map(|workflow| workflow.jobs.iter())
        .find(|job| job.id == id)
        .unwrap_or_else(|| panic!("workflow estate must define the `{id}` job"))
}

/// Returns a job's step that uses `action`, or panics naming both.
fn step_using<'a>(job: &'a Job, action: &str) -> &'a Step {
    job.step_using(action)
        .unwrap_or_else(|| panic!("`{}` must use the `{action}` action", job.id))
}

/// Asserts that a step supplies the expected value for one input.
fn assert_input(job_id: &str, step: &Step, key: &str, expected: &str) {
    assert_eq!(
        step.input(key),
        expected,
        "`{job_id}` step `{}` must set `{key}: {expected}`",
        step.label()
    );
}

#[rstest]
fn every_cache_reference_is_pinned_to_v6_1_0(workflows: Vec<Workflow>) {
    let unpinned: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with("actions/cache"))
        .filter(|(_, _, step)| !step.uses.ends_with(CACHE_ACTION_SHA))
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.uses))
        .collect();
    assert!(
        unpinned.is_empty(),
        "every actions/cache reference must pin {CACHE_ACTION_SHA} (v6.1.0): {unpinned:?}"
    );
}

#[rstest]
fn no_workflow_uses_the_ubicloud_cache_fork(workflows: Vec<Workflow>) {
    let forks: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with("ubicloud/cache"))
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.uses))
        .collect();
    assert!(
        forks.is_empty(),
        "the deprecated ubicloud/cache fork must not be used: {forks:?}"
    );
}

#[rstest]
fn every_shared_action_reference_is_pinned(workflows: Vec<Workflow>) {
    let mut references: Vec<String> = all_steps(&workflows)
        .into_iter()
        .map(|(file, job, step)| (file, job, step.uses))
        .chain(
            jobs(&workflows)
                .into_iter()
                .map(|(file, job)| (file, job.id.clone(), job.uses)),
        )
        .filter(|(_, _, uses)| uses.starts_with("leynos/shared-actions"))
        .filter(|(_, _, uses)| !uses.ends_with(SHARED_ACTIONS_SHA))
        .map(|(file, job, uses)| format!("{file}:{job}: {uses}"))
        .collect();
    references.sort();
    assert!(
        references.is_empty(),
        "every leynos/shared-actions reference must pin {SHARED_ACTIONS_SHA}: {references:?}"
    );
}

#[rstest]
fn no_step_builds_a_tool_from_source(workflows: Vec<Workflow>) {
    let offenders: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| {
            SOURCE_BUILD_FRAGMENTS
                .iter()
                .any(|fragment| step.run.contains(fragment))
        })
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.name))
        .collect();
    assert!(
        offenders.is_empty(),
        "tools must be installed from verified prebuilt releases, not compiled: {offenders:?}"
    );
}

#[rstest]
fn install_action_fails_closed_rather_than_compiling(workflows: Vec<Workflow>) {
    let permissive: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, _, step)| step.uses.starts_with("taiki-e/install-action"))
        .filter(|(_, _, step)| step.input("fallback") != "none")
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.name))
        .collect();
    assert!(
        permissive.is_empty(),
        "taiki-e/install-action must set `fallback: none` so it cannot compile: {permissive:?}"
    );
}

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

#[rstest]
fn non_build_jobs_stay_on_github_hosted_runners(workflows: Vec<Workflow>) {
    let misplaced: Vec<String> = jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| !job.runs_on.is_empty())
        .filter(|(_, job)| !BUILD_JOB_IDS.contains(&job.id.as_str()))
        .filter(|(_, job)| !job.is_github_hosted())
        .map(|(file, job)| format!("{file}:{}: {}", job.id, job.runs_on))
        .collect();
    assert!(
        misplaced.is_empty(),
        "delayed-comment, metadata, and other API-bound jobs must stay GitHub-hosted: {misplaced:?}"
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
        job.runs_on, UBICLOUD_LABEL,
        "`{id}` must keep its measured runner label"
    );
    let timeout = job
        .timeout_minutes
        .unwrap_or_else(|| panic!("`{id}` bills by the minute and must declare timeout-minutes"));
    assert!(
        (lowest..=highest).contains(&timeout),
        "`{id}` timeout-minutes {timeout} must lie between {lowest} and {highest}"
    );
}

#[rstest]
fn every_runner_label_is_registered_with_actionlint(workflows: Vec<Workflow>) {
    let text = read_repository_file(".github/actionlint.yaml")
        .unwrap_or_else(|err| panic!("actionlint configuration must be readable: {err}"));
    let unregistered: Vec<String> = jobs(&workflows)
        .into_iter()
        .filter(|(_, job)| !job.runs_on.is_empty() && !job.is_github_hosted())
        .filter(|(_, job)| !text.contains(job.runs_on.as_str()))
        .map(|(file, job)| format!("{file}:{}: {}", job.id, job.runs_on))
        .collect();
    assert!(
        unregistered.is_empty(),
        "every self-hosted label must appear in .github/actionlint.yaml: {unregistered:?}"
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
fn setup_rust_owns_the_cargo_registry_and_installs_no_compiler_cache(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, "setup-rust");
        assert_input(id, step, "cache-provider", "github");
        assert_input(id, step, "use-sccache", "false");
    }
}

#[rstest]
fn coverage_runs_the_whole_suite_once_and_owns_no_cargo_cache(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, "generate-coverage");
        for flag in ["all-features", "all-targets", "doctests"] {
            assert_input(id, step, flag, "true");
        }
        assert_input(id, step, "cache-provider", "external");
    }
}

#[rstest]
fn whitaker_is_installed_from_a_pinned_prebuilt_release(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    let step = step_using(job, "install-whitaker");
    assert_input("build-test", step, "installer-version", "0.2.7");
    assert_input("build-test", step, "cache-provider", "github");
}

#[rstest]
fn the_uv_cache_names_its_layers_and_keys_them_by_runner(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    let step = job
        .steps
        .iter()
        .find(|step| step.cache_paths().iter().any(|path| path == ".uv-cache"))
        .unwrap_or_else(|| panic!("`build-test` must cache the uv download layer"));
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

#[rstest]
#[case::not_a_workflow("scratch.yml", "steps: []")]
#[case::mistyped_runner("scratch.yml", "jobs:\n  a:\n    runs-on: [a, b]\n")]
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
fn a_malformed_workflow_is_an_error_not_a_default(#[case] file: &str, #[case] text: &str) {
    let outcome = parse_workflow(file, text);
    assert!(
        outcome.is_err(),
        "a workflow of unexpected shape must be rejected, not silently defaulted"
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
