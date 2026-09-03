//! Cache-ownership model for the repository's workflow jobs.
//!
//! Every mutable path a job caches must have exactly one owner. Some owners
//! are `actions/cache` steps in this repository; others are shared composite
//! actions that cache on the caller's behalf unless told that the caller owns
//! the path. This module reduces both kinds to the same `(path, owner)` list
//! so one contract can compare them.
//!
//! # Examples
//!
//! ```no_run
//! let owners = workflow_cache_owners::owners_for(&job);
//! assert!(owners.iter().all(|owner| !owner.path.is_empty()));
//! ```

use std::collections::BTreeMap;

use crate::workflow_model::{Job, Step};

/// A single claim that one step caches one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheOwner {
    /// Cached path, as written in the workflow or the shared action.
    pub path: String,
    /// Step that claims the path, named for a readable assertion message.
    pub owner: String,
}

/// Paths a shared composite action caches when `cache-provider` is `github`.
///
/// These mirror the action definitions at
/// `leynos/shared-actions@7d46a399558914f5a05074e55a560fec0269fd0d`. A caller
/// that sets `cache-provider: external` takes the path away from the action,
/// which is how a second owner of the Cargo registry is avoided.
const SHARED_ACTION_CACHES: [(&str, &[&str]); 3] = [
    ("setup-rust", &["~/.cargo/registry", "~/.cargo/git"]),
    (
        "generate-coverage",
        &[
            "~/.cargo/bin/cargo-binstall",
            "~/.cargo/bin/cargo-llvm-cov",
            "~/.cargo/bin/cargo-nextest",
            "~/.cargo/registry",
            "~/.cargo/git",
        ],
    ),
    (
        "install-whitaker",
        &[
            "~/.cargo/bin/whitaker-installer",
            "~/.cargo/bin/.whitaker-installer-version",
            "~/.local/share/whitaker",
        ],
    ),
];

fn shared_action_name(uses: &str) -> Option<&str> {
    let path = uses.split('@').next()?;
    let name = path.strip_prefix("leynos/shared-actions/.github/actions/")?;
    Some(name)
}

fn is_cache_action(uses: &str) -> bool {
    uses.split('@')
        .next()
        .is_some_and(|path| path == "actions/cache" || path.starts_with("actions/cache/"))
}

fn step_label(step: &Step) -> String {
    if step.name.is_empty() {
        step.uses.clone()
    } else {
        step.name.clone()
    }
}

fn direct_owners(step: &Step) -> Vec<CacheOwner> {
    if !is_cache_action(&step.uses) {
        return Vec::new();
    }
    let owner = step_label(step);
    step.cache_paths()
        .into_iter()
        .map(|path| CacheOwner {
            path,
            owner: owner.clone(),
        })
        .collect()
}

fn shared_owners(step: &Step) -> Vec<CacheOwner> {
    let Some(name) = shared_action_name(&step.uses) else {
        return Vec::new();
    };
    // An empty input means the action's default, which is `github` for every
    // shared action this repository calls.
    let provider = step.input("cache-provider");
    if !provider.is_empty() && provider != "github" {
        return Vec::new();
    }
    let owner = step_label(step);
    SHARED_ACTION_CACHES
        .iter()
        .filter(|(action, _)| *action == name)
        .flat_map(|(_, paths)| paths.iter())
        .map(|path| CacheOwner {
            path: (*path).to_owned(),
            owner: owner.clone(),
        })
        .collect()
}

/// Returns every cache claim made by a job, in step order.
///
/// `actions/cache/restore` and `actions/cache/save` halves of one split cache
/// share a step name prefix in practice; they are reported separately and the
/// caller decides whether the pair is a duplicate.
#[must_use]
pub fn owners_for(job: &Job) -> Vec<CacheOwner> {
    job.steps
        .iter()
        .flat_map(|step| {
            let mut claims = direct_owners(step);
            claims.extend(shared_owners(step));
            claims
        })
        .collect()
}

/// Returns the paths a job caches under more than one owner.
#[must_use]
pub fn duplicated_paths(job: &Job) -> Vec<(String, Vec<String>)> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for claim in owners_for(job) {
        let owners = grouped.entry(claim.path).or_default();
        if !owners.contains(&claim.owner) {
            owners.push(claim.owner);
        }
    }
    grouped
        .into_iter()
        .filter(|(_, owners)| owners.len() > 1)
        .collect()
}
