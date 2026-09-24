//! The workflow shapes the property tests and the contracts both reason about.
//!
//! A job, its steps, and how it selects a runner. Everything needed to load
//! workflows from disk, and everything only the contracts ask for, lives in
//! `workflow_estate.rs` instead, so a test binary that needs only these types
//! does not pull in a module of items it never names.
//!
//! # Examples
//!
//! ```no_run
//! let job = workflow_model::Job::default();
//! assert!(!job.is_github_hosted());
//! assert!(!job.runs_on.names_a_runner());
//! ```

use std::collections::BTreeMap;

use crate::runner_selection::{is_hosted_label, is_hosted_ubuntu_label, RunnerSelection};

/// An `if` or `continue-on-error` value that is constantly false.
///
/// A workflow may write either as a bare YAML boolean or as an expression, so
/// the rendered scalar is compared against every spelling of a constant that
/// decides the same way every time the workflow runs.
///
/// The empty string is one of them. GitHub evaluates an empty condition as
/// falsy, so `if:` with nothing after it is a scope that never runs, and the
/// model keeps a present-but-empty condition apart from an absent one for
/// exactly this reason: an absent `if` is a scope that always runs.
const CONSTANT_FALSE: [&str; 4] = ["false", "${{ false }}", "${{false}}", ""];

/// An `if` or `continue-on-error` value that is constantly true.
const CONSTANT_TRUE: [&str; 3] = ["true", "${{ true }}", "${{true}}"];

/// Reports whether a rendered condition is a constant that never holds.
///
/// Read on `if`, where it means the scope is dead: every rule about how the
/// scope is configured stays satisfiable while nothing it configures ever
/// executes. A guard that cannot run is worse than no guard, because the
/// contract above it goes on passing.
#[must_use]
pub fn is_constantly_false(condition: &str) -> bool {
    CONSTANT_FALSE.contains(&condition.trim())
}

/// Reports whether a rendered condition is a constant that always holds.
///
/// Read on `continue-on-error`, where it means the scope's result is advisory:
/// the job reports success whatever it found, so every budget and placement
/// rule about it describes a lane that cannot fail.
#[must_use]
pub fn is_constantly_true(condition: &str) -> bool {
    CONSTANT_TRUE.contains(&condition.trim())
}

/// One step of a workflow job, reduced to the fields the contracts inspect.
#[derive(Debug, Clone, Default)]
pub struct Step {
    /// Display name, or an empty string when the step is unnamed.
    pub name: String,
    /// Action reference, or an empty string for a `run` step.
    pub uses: String,
    /// Shell script, or an empty string for a `uses` step.
    pub run: String,
    /// Declared `if`, when present, rendered as written.
    ///
    /// Absence and an empty string are kept apart: the first is a step that
    /// always runs, the second a condition GitHub reads as false.
    pub condition: Option<String>,
    /// Declared `continue-on-error`, when present, rendered as written.
    pub continue_on_error: Option<String>,
    /// Inputs supplied to the action, rendered as GitHub would pass them.
    pub with: BTreeMap<String, String>,
}

impl Step {
    /// Reports whether the step's `if` is a constant that never holds.
    #[must_use]
    pub fn never_runs(&self) -> bool {
        self.condition.as_deref().is_some_and(is_constantly_false)
    }

    /// Reports whether the step's failures are advisory rather than fatal.
    #[must_use]
    pub fn result_is_advisory(&self) -> bool {
        self.continue_on_error
            .as_deref()
            .is_some_and(is_constantly_true)
    }

    /// Returns the value of a `with` input, or an empty string when absent.
    ///
    /// Every input was validated as a scalar during parsing, so an absent
    /// input and a mistyped one cannot be confused here.
    #[must_use]
    pub fn input(&self, key: &str) -> &str {
        self.with.get(key).map_or("", String::as_str)
    }

    /// Returns the newline-separated `path` input as individual entries.
    #[must_use]
    pub fn cache_paths(&self) -> Vec<String> {
        self.input("path")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }

    /// Returns the step's display name, falling back to its action reference.
    #[must_use]
    pub const fn label(&self) -> &str {
        if self.name.is_empty() {
            self.uses.as_str()
        } else {
            self.name.as_str()
        }
    }
}

/// One job of a workflow, reduced to the fields the contracts inspect.
#[derive(Debug, Clone, Default)]
pub struct Job {
    /// Key under the workflow's `jobs` mapping.
    pub id: String,
    /// How the job selects its runner.
    pub runs_on: RunnerSelection,
    /// Reusable workflow reference, or an empty string for a normal job.
    pub uses: String,
    /// Declared `timeout-minutes`, when present.
    pub timeout_minutes: Option<u64>,
    /// Declared `if`, when present, rendered as written.
    ///
    /// Modelled because placement and budget rules are all statements about a
    /// job that runs. A job carrying `if: false` keeps a valid `runs_on`, a
    /// bounded `timeout-minutes` and a correct cache key while executing
    /// nothing, so every contract about it passes and none of them is true of
    /// anything.
    pub condition: Option<String>,
    /// Declared `continue-on-error`, when present, rendered as written.
    ///
    /// The same argument one step further on: a lane whose failures do not
    /// count is a lane the rules describe and the gate cannot enforce.
    pub continue_on_error: Option<String>,
    /// Job-level environment, rendered as GitHub would export it.
    pub env: BTreeMap<String, String>,
    /// Steps in declaration order.
    pub steps: Vec<Step>,
}

impl Job {
    /// Returns a job-level environment value, or an empty string when unset.
    #[must_use]
    pub fn env(&self, key: &str) -> &str {
        self.env.get(key).map_or("", String::as_str)
    }

    /// Reports whether the job runs on a runner GitHub hosts, of any family.
    ///
    /// A runner group is never GitHub-hosted, and a label set is only when
    /// every label in it is one of GitHub's images: a job that also requires a
    /// self-hosted label runs somewhere else.
    ///
    /// This answers who pays for the runner. For where a job is allowed to
    /// sit, ask `stays_on_hosted_ubuntu` instead: the placement rule names one
    /// family, and the two questions differ by exactly the jobs that would
    /// move to Windows or macOS.
    #[must_use]
    pub fn is_github_hosted(&self) -> bool {
        self.every_label_is(is_hosted_label)
    }

    /// Reports whether the job sits on a GitHub-hosted Ubuntu runner.
    ///
    /// The placement rule keeps every lane that is not a measured build on
    /// `ubuntu-latest`, so a job that satisfies `is_github_hosted` on a
    /// Windows or macOS image does not satisfy this. They were the same
    /// predicate, and the looser one was the one the contract read.
    #[must_use]
    pub fn stays_on_hosted_ubuntu(&self) -> bool {
        self.every_label_is(is_hosted_ubuntu_label)
    }

    /// Reports whether the job names labels and every one of them satisfies
    /// `classify`.
    ///
    /// A fork-fallback selection holds one hosted arm and one that is not, so
    /// it is never wholly hosted: it runs on this repository's own runner for
    /// every branch of this repository.
    fn every_label_is(&self, classify: fn(&str) -> bool) -> bool {
        match &self.runs_on {
            RunnerSelection::Labels(labels) => {
                !labels.is_empty() && labels.iter().all(|label| classify(label.as_str()))
            }
            RunnerSelection::Delegated
            | RunnerSelection::Group { .. }
            | RunnerSelection::ForkFallback { .. } => false,
        }
    }

    /// Reports whether the job's own `if` is a constant that never holds.
    #[must_use]
    pub fn never_runs(&self) -> bool {
        self.condition.as_deref().is_some_and(is_constantly_false)
    }

    /// Reports whether the job's failures are advisory rather than fatal.
    #[must_use]
    pub fn result_is_advisory(&self) -> bool {
        self.continue_on_error
            .as_deref()
            .is_some_and(is_constantly_true)
    }

    /// Returns the first step whose `run` or `uses` text contains `needle`.
    #[must_use]
    pub fn first_step_containing(&self, needle: &str) -> Option<usize> {
        self.steps
            .iter()
            .position(|step| step.run.contains(needle) || step.uses.contains(needle))
    }

    /// Returns the first step matching `needle`, with its index.
    #[must_use]
    pub fn first_step_with(&self, needle: &str) -> Option<(usize, &Step)> {
        self.steps
            .iter()
            .enumerate()
            .find(|(_, step)| step.run.contains(needle) || step.uses.contains(needle))
    }

    /// Returns the first step whose `uses` is `coordinate`, ignoring its pin.
    ///
    /// `coordinate` is the whole reference before the `@`, publisher included.
    /// A suffix match would accept `untrusted/setup-rust@<sha>` wherever the
    /// contracts ask for the shared `setup-rust`, so an action from the wrong
    /// publisher could satisfy a policy check written to exclude it.
    #[must_use]
    pub fn step_using(&self, coordinate: &str) -> Option<&Step> {
        self.steps
            .iter()
            .find(|step| step.uses.split('@').next() == Some(coordinate))
    }
}
