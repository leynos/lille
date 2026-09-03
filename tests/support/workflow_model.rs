//! Types describing the repository's GitHub Actions workflow estate.
//!
//! The workflow-contract tests assert placement, tool-install, and cache
//! ownership rules over these types rather than over raw YAML text, so a
//! reordered key or a reflowed block scalar cannot silently defeat a rule.
//! `workflow_loader` turns files into these values; this module holds only
//! the shapes and the queries the contracts ask of them.
//!
//! # Examples
//!
//! ```no_run
//! let job = workflow_model::Job::default();
//! assert!(!job.is_github_hosted());
//! ```

use std::{collections::BTreeMap, fmt};

/// Directory holding the repository's workflow definitions.
pub const WORKFLOW_DIR: &str = ".github/workflows";

/// Commit that every `actions/cache` reference must pin (v6.1.0).
pub const CACHE_ACTION_SHA: &str = "55cc8345863c7cc4c66a329aec7e433d2d1c52a9";

/// Commit that every `leynos/shared-actions` reference must pin.
pub const SHARED_ACTIONS_SHA: &str = "7d46a399558914f5a05074e55a560fec0269fd0d";

/// Runner label used by this repository's Ubicloud build and test jobs.
pub const UBICLOUD_LABEL: &str = "ubicloud-standard-8";

/// Jobs that build or test the crate and therefore keep an Ubicloud label.
pub const BUILD_JOB_IDS: [&str; 2] = ["build-test", "coverage-upload"];

/// Failure encountered while reading or parsing the workflow estate.
#[derive(Debug)]
pub enum WorkflowError {
    /// A workflow file or the workflow directory could not be read.
    Read(String, std::io::Error),
    /// A workflow file was not valid YAML.
    Parse(String, serde_norway::Error),
    /// A workflow file was structurally unusable.
    Shape(String, String),
}

impl fmt::Display for WorkflowError {
    /// Renders the failure with the workflow name that produced it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(name, err) => write!(f, "cannot read {name}: {err}"),
            Self::Parse(name, err) => write!(f, "cannot parse {name}: {err}"),
            Self::Shape(name, msg) => write!(f, "unexpected shape in {name}: {msg}"),
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Where in the estate a value was read, carried instead of a bare string so
/// the parsing helpers take one string argument rather than several.
#[derive(Debug, Clone)]
pub struct Location(String);

impl Location {
    /// Locates a whole workflow file.
    #[must_use]
    pub fn file(name: &str) -> Self {
        Self(name.to_owned())
    }

    /// Locates one job within this file.
    #[must_use]
    pub fn job(&self, id: &str) -> Self {
        Self(format!("{}: job `{id}`", self.0))
    }

    /// Builds a shape error reported at this location.
    #[must_use]
    pub fn shape(&self, message: &str) -> WorkflowError {
        WorkflowError::Shape(self.0.clone(), message.to_owned())
    }
}

/// A workflow document paired with the file name it came from.
#[derive(Debug, Clone, Copy)]
pub struct WorkflowSource<'a> {
    /// File name within [`WORKFLOW_DIR`].
    pub file: &'a str,
    /// The document's YAML text.
    pub text: &'a str,
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
    /// Runner label, or an empty string when the job calls a reusable workflow.
    pub runs_on: String,
    /// Reusable workflow reference, or an empty string for a normal job.
    pub uses: String,
    /// Declared `timeout-minutes`, when present.
    pub timeout_minutes: Option<u64>,
    /// Steps in declaration order.
    pub steps: Vec<Step>,
}

impl Job {
    /// Reports whether the job runs on a GitHub-hosted Ubuntu runner.
    #[must_use]
    pub fn is_github_hosted(&self) -> bool {
        self.runs_on.starts_with("ubuntu-")
    }

    /// Returns the first step whose `run` or `uses` text contains `needle`.
    #[must_use]
    pub fn first_step_containing(&self, needle: &str) -> Option<usize> {
        self.steps
            .iter()
            .position(|step| step.run.contains(needle) || step.uses.contains(needle))
    }

    /// Returns the first step whose `uses` names `action`, ignoring its pin.
    #[must_use]
    pub fn step_using(&self, action: &str) -> Option<&Step> {
        self.steps.iter().find(|step| {
            step.uses
                .split('@')
                .next()
                .is_some_and(|path| path.ends_with(action))
        })
    }
}

/// One workflow file.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// File name within [`WORKFLOW_DIR`].
    pub file: String,
    /// Jobs in declaration order.
    pub jobs: Vec<Job>,
}
