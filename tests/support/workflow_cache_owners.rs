//! Cache-ownership model for the repository's workflow jobs.
//!
//! Every mutable path a job caches must have exactly one owner. Some owners
//! are `actions/cache` steps in this repository; others are shared composite
//! actions that cache on the caller's behalf unless told that the caller owns
//! the path. This module reduces both kinds to the same `(path, owner)` list
//! so one contract can compare them.
//!
//! Owner identity is the step's position in its job, never its display name:
//! two steps may legitimately share a name, and collapsing them would hide a
//! duplicate owner. The one deliberate exception is a split cache, where an
//! `actions/cache/restore` step and an `actions/cache/save` step that share a
//! key are the two halves of a single owner.
//!
//! # Examples
//!
//! ```no_run
//! let owners = workflow_cache_owners::owners_for(&job);
//! assert!(owners.iter().all(|owner| !owner.path.is_empty()));
//! ```

use std::collections::{BTreeMap, BTreeSet};

use crate::workflow_model::{Job, Step};

/// A single claim that one step caches one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheOwner {
    /// Cached path, as written in the workflow or the shared action.
    pub path: String,
    /// Identity of the claiming owner, unique per step or per split-cache key.
    pub owner: String,
}

/// Paths a shared composite action caches when `cache-provider` is `github`.
///
/// These mirror the action definitions at
/// `leynos/shared-actions@c6125f19593668cbfefd65a59c08cb7aefe90d93`. A caller
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

/// Returns the shared-action name a `uses` reference names, if any.
fn shared_action_name(uses: &str) -> Option<&str> {
    uses.split('@')
        .next()?
        .strip_prefix("leynos/shared-actions/.github/actions/")
}

fn action_path(uses: &str) -> &str {
    uses.split('@').next().unwrap_or_default()
}

/// The half of a split cache a step is, if it is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitHalf {
    /// An `actions/cache/restore` step.
    Restore,
    /// An `actions/cache/save` step.
    Save,
}

/// Returns which half of a split cache a step is, or `None` for other steps.
fn split_half(uses: &str) -> Option<SplitHalf> {
    match action_path(uses) {
        "actions/cache/restore" => Some(SplitHalf::Restore),
        "actions/cache/save" => Some(SplitHalf::Save),
        _ => None,
    }
}

/// Identity of the owner making a claim.
///
/// A whole-cache or shared-action claim is owned by its step alone. A split
/// cache is owned jointly by the one restore and the one save step that share
/// its key, so those two halves report the same identity and are not counted
/// twice.
///
/// `paired` says whether this step is half of exactly such a pair. Two
/// restores sharing a key are two owners, not one, and so are a restore, a
/// save, and a second restore: only a step with exactly one counterpart of the
/// other half may share an identity with it. Without that condition a
/// duplicate claim could hide behind the split-cache exception.
fn owner_identity(step: &Step, index: usize, paired: bool) -> String {
    let label = if step.name.is_empty() {
        step.uses.as_str()
    } else {
        step.name.as_str()
    };
    if paired {
        return format!("split cache with key `{}`", step.input("key"));
    }
    format!("step {index} (`{label}`)")
}

/// Returns the claims an `actions/cache` step makes on its own `path` input.
fn direct_owners(step: &Step, index: usize, paired: bool) -> Vec<CacheOwner> {
    if !action_path(&step.uses).starts_with("actions/cache") {
        return Vec::new();
    }
    let owner = owner_identity(step, index, paired);
    step.cache_paths()
        .into_iter()
        .map(|path| CacheOwner {
            path,
            owner: owner.clone(),
        })
        .collect()
}

/// Returns the claims a shared composite action makes on the caller's behalf.
fn shared_owners(step: &Step, index: usize) -> Vec<CacheOwner> {
    let Some(name) = shared_action_name(&step.uses) else {
        return Vec::new();
    };
    // An empty input means the action's default, which is `github` for every
    // shared action this repository calls.
    let provider = step.input("cache-provider");
    if !provider.is_empty() && provider != "github" {
        return Vec::new();
    }
    let owner = owner_identity(step, index, false);
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

/// Returns the keys for which exactly one restore step and one save step exist.
///
/// Only those keys join their two steps into a single owner. A key claimed by
/// two restores, or by a pair plus a third step, leaves every one of its steps
/// an owner in its own right, which is what makes the duplicate visible.
fn paired_split_keys(job: &Job) -> BTreeSet<String> {
    let mut halves: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for step in &job.steps {
        let Some(half) = split_half(&step.uses) else {
            continue;
        };
        let counts = halves.entry(step.input("key").to_owned()).or_default();
        match half {
            SplitHalf::Restore => counts.0 += 1,
            SplitHalf::Save => counts.1 += 1,
        }
    }
    halves
        .into_iter()
        .filter(|(_, (restores, saves))| *restores == 1 && *saves == 1)
        .map(|(key, _)| key)
        .collect()
}

/// Returns every cache claim made by a job, in step order.
#[must_use]
pub fn owners_for(job: &Job) -> Vec<CacheOwner> {
    let paired = paired_split_keys(job);
    job.steps
        .iter()
        .enumerate()
        .flat_map(|(index, step)| {
            let is_paired = split_half(&step.uses).is_some() && paired.contains(step.input("key"));
            let mut claims = direct_owners(step, index, is_paired);
            claims.extend(shared_owners(step, index));
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
