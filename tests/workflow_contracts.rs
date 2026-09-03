//! Structural contracts over the repository's GitHub Actions workflows.
//!
//! These tests encode the Ubicloud adoption rules that a reviewer would
//! otherwise have to re-check by hand on every workflow edit: no tool is built
//! from source, every cached path has exactly one owner, cache and shared
//! action references are pinned, API-bound jobs stay GitHub-hosted, and an
//! installer always precedes the first use of what it installs.
//!
//! They read the workflow files directly, so they fail on the change that
//! introduces a violation rather than on the CI run that suffers from it.

#[path = "support/workflow_cache_owners.rs"]
mod workflow_cache_owners;
#[path = "support/workflow_model.rs"]
mod workflow_model;

use std::{fs, path::PathBuf};

use rstest::{fixture, rstest};

use workflow_model::{
    all_steps, load_workflows, Job, Workflow, BUILD_JOB_IDS, CACHE_ACTION_SHA, SHARED_ACTIONS_SHA,
    UBICLOUD_LABEL,
};

/// Prefixes that mark a step as building a tool from source.
///
/// `cargo binstall` is included because it compiles whenever its default
/// strategies fall through to `compile`; the estate's rule is to install from
/// a verified release archive instead.
const SOURCE_BUILD_FRAGMENTS: [&str; 3] = ["cargo install", "cargo-binstall ", "cargo binstall"];

/// Every workflow in `.github/workflows`, parsed once per test.
#[fixture]
fn workflows() -> Vec<Workflow> {
    load_workflows().unwrap_or_else(|err| panic!("workflow estate must parse: {err}"))
}

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

fn job_named<'a>(workflows: &'a [Workflow], id: &str) -> &'a Job {
    workflows
        .iter()
        .flat_map(|workflow| workflow.jobs.iter())
        .find(|job| job.id == id)
        .unwrap_or_else(|| panic!("workflow estate must define the `{id}` job"))
}

fn first_step_index(job: &Job, predicate: impl Fn(&str) -> bool) -> Option<usize> {
    job.steps
        .iter()
        .position(|step| predicate(&step.run) || predicate(&step.uses))
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

#[rstest]
fn build_jobs_keep_their_ubicloud_label_and_a_timeout(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        assert_eq!(
            job.runs_on, UBICLOUD_LABEL,
            "`{id}` must keep its measured runner label"
        );
        assert!(
            job.timeout_minutes.is_some(),
            "`{id}` bills by the minute and must declare timeout-minutes"
        );
    }
}

#[rstest]
fn every_runner_label_is_registered_with_actionlint(workflows: Vec<Workflow>) {
    let config = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/actionlint.yaml");
    let text = fs::read_to_string(&config).unwrap_or_default();
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
        let install_at = first_step_index(job, |text| text.contains(installer));
        let use_at = first_step_index(job, |text| text.contains(first_use));
        let Some(use_index) = use_at else { continue };
        let Some(install_index) = install_at else {
            panic!("`{id}` uses `{first_use}` without a `{installer}` step");
        };
        assert!(
            install_index < use_index,
            "`{id}` must run `{installer}` before step {use_index} uses `{first_use}`"
        );
    }
}

#[rstest]
fn coverage_is_the_only_linux_test_execution(workflows: Vec<Workflow>) {
    let duplicates: Vec<String> = all_steps(&workflows)
        .into_iter()
        .filter(|(_, job, _)| BUILD_JOB_IDS.contains(&job.as_str()))
        .filter(|(_, _, step)| {
            step.run.contains("cargo test") || step.run.contains("cargo nextest")
        })
        .map(|(file, job, step)| format!("{file}:{job}: {}", step.name))
        .collect();
    assert!(
        duplicates.is_empty(),
        "the instrumented coverage run is the only test execution; drop the repeat: {duplicates:?}"
    );
}
