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

use std::{collections::BTreeMap, fmt};

/// How a job selects the runner it executes on.
///
/// GitHub Actions accepts three shapes for `runs-on`: a single label, a
/// sequence of labels a runner must carry all of, and a mapping naming a
/// runner group with optional labels. Modelling only the scalar would make the
/// other two shapes parse errors, so a perfectly valid workflow would fail the
/// contracts instead of the workflow that deserves to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum RunnerSelection {
    /// The job names no runner because it calls a reusable workflow.
    #[default]
    Delegated,
    /// Labels a runner must carry, from a scalar or a sequence.
    Labels(Vec<String>),
    /// A runner group, with the labels required within that group.
    Group {
        /// Name of the runner group.
        group: String,
        /// Labels required within the group, possibly empty.
        labels: Vec<String>,
    },
}

impl RunnerSelection {
    /// Returns the labels the selection requires, empty when it names none.
    #[must_use]
    pub fn labels(&self) -> &[String] {
        match self {
            Self::Delegated => &[],
            Self::Labels(labels) | Self::Group { labels, .. } => labels,
        }
    }

    /// Reports whether the job names a runner of its own.
    #[must_use]
    pub const fn names_a_runner(&self) -> bool {
        !matches!(self, Self::Delegated)
    }
}

impl fmt::Display for RunnerSelection {
    /// Renders the selection the way a failure message should quote it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Delegated => write!(f, "(reusable workflow)"),
            Self::Labels(labels) => write!(f, "{}", labels.join(", ")),
            Self::Group { group, labels } if labels.is_empty() => write!(f, "group {group}"),
            Self::Group { group, labels } => {
                write!(f, "group {group} ({})", labels.join(", "))
            }
        }
    }
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
    /// Inputs supplied to the action, rendered as GitHub would pass them.
    pub with: BTreeMap<String, String>,
}

impl Step {
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

    /// Reports whether the job runs on a GitHub-hosted Ubuntu runner.
    ///
    /// A runner group is never GitHub-hosted, and a label set is only when
    /// every label in it is one of GitHub's Ubuntu images: a job that also
    /// requires a self-hosted label runs somewhere else.
    #[must_use]
    pub fn is_github_hosted(&self) -> bool {
        match &self.runs_on {
            RunnerSelection::Labels(labels) => {
                !labels.is_empty() && labels.iter().all(|label| label.starts_with("ubuntu-"))
            }
            RunnerSelection::Delegated | RunnerSelection::Group { .. } => false,
        }
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
