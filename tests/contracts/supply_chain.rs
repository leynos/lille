//! Supply-chain contracts over the workflow estate.
//!
//! Every third-party reference is pinned to a commit, and every tool arrives
//! as a verified prebuilt release. These are the rules that decide what code
//! the estate is willing to execute, so a violation is a trust question rather
//! than a performance one.

use rstest::rstest;

use crate::shared_action;
use crate::workflow_assertions::{assert_input, job_named, jobs, step_using, workflows};
use crate::workflow_cache_owners::is_cache_action;
use crate::workflow_estate::{Workflow, BUILD_JOB_IDS, CACHE_ACTION_SHA, SHARED_ACTIONS_SHA};
use crate::workflow_loader::all_steps;

/// Fragments that mark a step as building a tool from source.
///
/// `cargo binstall` is included because it compiles whenever its default
/// strategies fall through to `compile`; the estate's rule is to install from
/// a verified release archive instead.
const SOURCE_BUILD_FRAGMENTS: [&str; 3] = ["cargo install", "cargo-binstall ", "cargo binstall"];

/// Pinned prebuilt sccache the build jobs install.
const SCCACHE_TOOL: &str = "sccache@0.16.0";

#[rstest]
fn every_cache_reference_is_pinned_to_v6_1_0(workflows: Vec<Workflow>) {
    let unpinned: Vec<String> = all_steps(&workflows)
        .into_iter()
        // Matched on the exact coordinate: `actions/cache-audit` shares the
        // prefix and is a different action, which this rule has nothing to say
        // about.
        .filter(|(_, _, step)| is_cache_action(&step.uses))
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
fn sccache_is_installed_from_a_pinned_prebuilt_release(workflows: Vec<Workflow>) {
    for id in BUILD_JOB_IDS {
        let job = job_named(&workflows, id);
        let step = step_using(job, "taiki-e/install-action");
        assert_input(id, step, "tool", SCCACHE_TOOL);
        assert_input(id, step, "fallback", "none");
    }
}

#[rstest]
fn whitaker_is_installed_from_a_pinned_prebuilt_release(workflows: Vec<Workflow>) {
    let job = job_named(&workflows, "build-test");
    let step = step_using(job, &shared_action("install-whitaker"));
    assert_input("build-test", step, "installer-version", "0.2.7");
    assert_input("build-test", step, "cache-provider", "github");
}
