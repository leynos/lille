//! Sampled properties over the workflow model's ownership and ordering rules.
//!
//! The deterministic contracts in `workflow_contracts.rs` check the current
//! workflow files. They cannot show that the cache-ownership and step-ordering
//! logic behaves over the wider domain of jobs a future edit could produce:
//! arbitrary step orderings, repeated display names, interleaved unrelated
//! steps, and split caches whose halves agree or disagree on a key. Per
//! `docs/adr-003-bounded-rstest-over-property-testing.md`, `proptest`
//! supplements the bounded matrices for exactly that kind of broader domain.
//!
//! Each property is checked against a small oracle expressed independently of
//! the implementation, rather than by re-deriving the implementation's answer.

#[path = "support/workflow_cache_owners.rs"]
mod workflow_cache_owners;
#[path = "support/workflow_model.rs"]
// The model is shared with tests/workflow_contracts.rs, which exercises the
// loading and contract-facing half. These properties need only the job and
// step types, so the rest is unused in this binary alone.
#[expect(
    dead_code,
    reason = "shared support module; the contracts binary uses the rest"
)]
mod workflow_model;

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use workflow_cache_owners::duplicated_paths;
use workflow_model::{Job, Step};

/// Cache paths the generators draw from, kept small so collisions are common.
const PATHS: [&str; 4] = ["~/.cargo/registry", "~/.cargo/git", ".uv-cache", "target-x"];

/// Display names the generators draw from, including deliberate repeats.
const NAMES: [&str; 3] = ["Cache", "Cache", "Restore"];

/// Builds a step that uses an action with the given inputs.
fn action_step(name: &str, uses: &str, inputs: &[(&str, &str)]) -> Step {
    Step {
        name: name.to_owned(),
        uses: uses.to_owned(),
        run: String::new(),
        with: inputs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<String, String>>(),
    }
}

/// Builds a whole-cache step claiming one path.
fn cache_step(name: &str, path: &str) -> Step {
    action_step(name, "actions/cache@sha", &[("path", path)])
}

/// Builds one half of a split cache claiming one path under one key.
fn split_step(half: &str, path: &str, key: &str) -> Step {
    action_step(
        "Split",
        &format!("actions/cache/{half}@sha"),
        &[("path", path), ("key", key)],
    )
}

/// Builds a step that runs a shell command and caches nothing.
fn run_step(script: &str) -> Step {
    Step {
        run: script.to_owned(),
        ..Step::default()
    }
}

/// Wraps steps in a job that satisfies the model's shape requirements.
fn job_of(steps: Vec<Step>) -> Job {
    Job {
        id: "j".to_owned(),
        runs_on: "ubuntu-latest".to_owned(),
        steps,
        ..Job::default()
    }
}

/// Generates a step that either caches a path or does unrelated work.
fn any_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        (
            prop::sample::select(NAMES.to_vec()),
            prop::sample::select(PATHS.to_vec()),
        )
            .prop_map(|(name, path)| cache_step(name, path)),
        any::<bool>().prop_map(|flag| run_step(if flag { "make lint" } else { "echo hello" })),
    ]
}

/// Returns the set of paths claimed more than once, ignoring owner identity.
///
/// This oracle counts claiming steps directly, so it is independent of how
/// the implementation names an owner.
fn paths_claimed_twice(job: &Job) -> BTreeSet<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for step in &job.steps {
        if step.uses.starts_with("actions/cache@") {
            for path in step.cache_paths() {
                *seen.entry(path).or_default() += 1;
            }
        }
    }
    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(path, _)| path)
        .collect()
}

/// Drops filler steps that would themselves claim the path under test.
///
/// The filler exists to prove that unrelated steps between two claims do not
/// disturb the result; a filler that claims the same path would instead test
/// a different scenario.
fn without_claims_on(steps: Vec<Step>, path: &str) -> Vec<Step> {
    steps
        .into_iter()
        .filter(|step| !step.cache_paths().iter().any(|claimed| claimed == path))
        .collect()
}

/// Returns the reported duplicated paths as a set.
fn reported(job: &Job) -> BTreeSet<String> {
    duplicated_paths(job)
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}

proptest! {
    /// Two whole-cache steps claiming a path are always two owners, whatever
    /// their display names, positions, or the steps interleaved between them.
    #[test]
    fn repeated_whole_cache_claims_are_always_duplicates(steps in prop::collection::vec(any_step(), 0..8)) {
        let job = job_of(steps);
        prop_assert_eq!(reported(&job), paths_claimed_twice(&job));
    }

    /// Reordering a job's steps cannot change which paths have two owners.
    #[test]
    fn duplicate_detection_ignores_step_order(
        steps in prop::collection::vec(any_step(), 0..8),
        rotation in 0usize..8,
    ) {
        let job = job_of(steps.clone());
        let mut rotated = steps;
        let count = rotated.len();
        if count > 0 {
            rotated.rotate_left(rotation % count);
        }
        prop_assert_eq!(reported(&job), reported(&job_of(rotated)));
    }

    /// A restore and a save sharing a key are one owner; differing keys are two.
    #[test]
    fn a_split_cache_is_one_owner_only_when_its_halves_agree(
        path in prop::sample::select(PATHS.to_vec()),
        same_key in any::<bool>(),
        filler in prop::collection::vec(any_step(), 0..4),
    ) {
        let save_key = if same_key { "k1" } else { "k2" };
        let mut steps = vec![split_step("restore", path, "k1")];
        steps.extend(without_claims_on(filler, path));
        steps.push(split_step("save", path, save_key));
        let job = job_of(steps);
        prop_assert_eq!(reported(&job).contains(path), !same_key);
    }

    /// A shared action is an owner exactly when the caller has not taken its
    /// paths with `cache-provider: external`.
    #[test]
    fn an_external_cache_provider_removes_the_shared_action_as_an_owner(
        external in any::<bool>(),
        filler in prop::collection::vec(any_step(), 0..4),
    ) {
        let provider = if external { "external" } else { "github" };
        let mut steps = vec![cache_step("Registry", "~/.cargo/registry")];
        steps.extend(without_claims_on(filler, "~/.cargo/registry"));
        steps.push(action_step(
            "Setup Rust",
            "leynos/shared-actions/.github/actions/setup-rust@sha",
            &[("cache-provider", provider)],
        ));
        let job = job_of(steps);
        prop_assert_eq!(reported(&job).contains("~/.cargo/registry"), !external);
    }

    /// `first_step_containing` always returns the least matching index.
    #[test]
    fn the_first_matching_step_is_the_least_matching_index(
        scripts in prop::collection::vec(prop::sample::select(vec!["whitaker --all", "cargo test", "echo"]), 0..8),
    ) {
        let job = job_of(scripts.iter().map(|script| run_step(script)).collect());
        let expected = job
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.run.contains("whitaker"))
            .map(|(index, _)| index)
            .min();
        prop_assert_eq!(job.first_step_containing("whitaker"), expected);
    }
}
