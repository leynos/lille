//! Structural model of the repository's GitHub Actions workflow files.
//!
//! The workflow-contract tests assert placement, tool-install, and cache
//! ownership rules over this model rather than over raw YAML text, so a
//! reordered key or a reflowed block scalar cannot silently defeat a rule.
//!
//! # Examples
//!
//! ```no_run
//! let workflows = workflow_model::load_workflows()?;
//! assert!(workflows.iter().any(|w| w.file == "ci.yml"));
//! # Ok::<(), workflow_model::WorkflowError>(())
//! ```

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use serde_norway::Value;

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
    /// A workflow file could not be read.
    Read(PathBuf, std::io::Error),
    /// A workflow file was not valid YAML.
    Parse(PathBuf, serde_norway::Error),
    /// A workflow file was structurally unusable.
    Shape(PathBuf, String),
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(path, err) => write!(f, "cannot read {}: {err}", path.display()),
            Self::Parse(path, err) => write!(f, "cannot parse {}: {err}", path.display()),
            Self::Shape(path, msg) => write!(f, "unexpected shape in {}: {msg}", path.display()),
        }
    }
}

impl std::error::Error for WorkflowError {}

/// One step of a workflow job, reduced to the fields the contracts inspect.
#[derive(Debug, Clone)]
pub struct Step {
    /// Display name, or an empty string when the step is unnamed.
    pub name: String,
    /// Action reference, or an empty string for a `run` step.
    pub uses: String,
    /// Shell script, or an empty string for a `uses` step.
    pub run: String,
    /// Inputs supplied to the action.
    pub with: Value,
}

impl Step {
    /// Returns the string value of a `with` input, or an empty string.
    #[must_use]
    pub fn input(&self, key: &str) -> String {
        self.with
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
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
}

/// One job of a workflow, reduced to the fields the contracts inspect.
#[derive(Debug, Clone)]
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
}

/// One workflow file.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// File name within [`WORKFLOW_DIR`].
    pub file: String,
    /// Jobs in declaration order.
    pub jobs: Vec<Job>,
}

fn workflow_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW_DIR)
}

fn scalar(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn parse_step(raw: &Value) -> Step {
    Step {
        name: scalar(raw, "name"),
        uses: scalar(raw, "uses"),
        run: scalar(raw, "run"),
        with: raw.get("with").cloned().unwrap_or(Value::Null),
    }
}

fn parse_job(id: &str, raw: &Value) -> Job {
    let steps = raw
        .get("steps")
        .and_then(Value::as_sequence)
        .map(|items| items.iter().map(parse_step).collect())
        .unwrap_or_default();
    Job {
        id: id.to_owned(),
        runs_on: scalar(raw, "runs-on"),
        uses: scalar(raw, "uses"),
        timeout_minutes: raw.get("timeout-minutes").and_then(Value::as_u64),
        steps,
    }
}

fn parse_workflow(path: &Path, text: &str) -> Result<Workflow, WorkflowError> {
    let document: Value = serde_norway::from_str(text)
        .map_err(|err| WorkflowError::Parse(path.to_path_buf(), err))?;
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or_else(|| {
            WorkflowError::Shape(path.to_path_buf(), "missing a `jobs` mapping".to_owned())
        })?;
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let parsed = jobs
        .iter()
        .map(|(id, raw)| parse_job(id.as_str().unwrap_or_default(), raw))
        .collect();
    Ok(Workflow { file, jobs: parsed })
}

/// Loads and parses every workflow in `.github/workflows`.
///
/// # Errors
///
/// Returns an error when the directory cannot be listed, a file cannot be
/// read, or a file is not a YAML document containing a `jobs` mapping.
pub fn load_workflows() -> Result<Vec<Workflow>, WorkflowError> {
    let dir = workflow_dir();
    let listing = fs::read_dir(&dir).map_err(|err| WorkflowError::Read(dir.clone(), err))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in listing {
        let path = entry
            .map_err(|err| WorkflowError::Read(dir.clone(), err))?
            .path();
        if path
            .extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text =
                fs::read_to_string(path).map_err(|err| WorkflowError::Read(path.clone(), err))?;
            parse_workflow(path.as_path(), &text)
        })
        .collect()
}

/// Returns every step of every job, tagged with its workflow and job.
#[must_use]
pub fn all_steps(workflows: &[Workflow]) -> Vec<(String, String, Step)> {
    workflows
        .iter()
        .flat_map(|workflow| {
            workflow.jobs.iter().flat_map(move |job| {
                job.steps
                    .iter()
                    .map(move |step| (workflow.file.clone(), job.id.clone(), step.clone()))
            })
        })
        .collect()
}
