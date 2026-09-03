//! Structural model of the repository's GitHub Actions workflow files.
//!
//! The workflow-contract tests assert placement, tool-install, and cache
//! ownership rules over this model rather than over raw YAML text, so a
//! reordered key or a reflowed block scalar cannot silently defeat a rule.
//!
//! Parsing is strict about shape. A field that is present but of the wrong
//! type is an error rather than a silent default, because a contract that
//! reads an empty string for a mistyped `runs-on` would pass a workflow it
//! should reject. Defaults are used only where a field is genuinely optional.
//!
//! Files are read through a `cap_std` directory capability rooted at
//! `.github/workflows`, so the loader cannot reach outside the workflow
//! directory even if a future contract passes it a name it should not.
//!
//! # Examples
//!
//! ```no_run
//! let workflows = workflow_model::load_workflows()?;
//! assert!(workflows.iter().any(|w| w.file == "ci.yml"));
//! # Ok::<(), workflow_model::WorkflowError>(())
//! ```

use std::{collections::BTreeMap, fmt};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
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

/// Builds a shape error naming the offending workflow and field.
fn shape(context: &str, message: &str) -> WorkflowError {
    WorkflowError::Shape(context.to_owned(), message.to_owned())
}

/// Renders a YAML scalar as the string a workflow expression would see.
///
/// GitHub Actions coerces booleans and numbers to strings when it passes an
/// input to an action, so `doctests: true` and `doctests: 'true'` reach the
/// action identically and must compare equal here too.
fn render_scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Reads an optional string field, defaulting to an empty string.
///
/// # Errors
///
/// Returns an error when the field is present but is not a scalar.
fn optional_string(raw: &Value, key: &str, context: &str) -> Result<String, WorkflowError> {
    let Some(value) = raw.get(key) else {
        return Ok(String::new());
    };
    render_scalar(value).ok_or_else(|| shape(context, &format!("`{key}` must be a scalar")))
}

/// Reads an optional unsigned integer field.
///
/// # Errors
///
/// Returns an error when the field is present but is not an unsigned integer.
fn optional_u64(raw: &Value, key: &str, context: &str) -> Result<Option<u64>, WorkflowError> {
    let Some(value) = raw.get(key) else {
        return Ok(None);
    };
    value
        .as_u64()
        .map(Some)
        .ok_or_else(|| shape(context, &format!("`{key}` must be an unsigned integer")))
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

/// Parses a step's `with` mapping into rendered scalar inputs.
///
/// # Errors
///
/// Returns an error when `with` is not a mapping or an input is not a scalar.
fn parse_inputs(raw: &Value, context: &str) -> Result<BTreeMap<String, String>, WorkflowError> {
    let Some(value) = raw.get("with") else {
        return Ok(BTreeMap::new());
    };
    let mapping = value
        .as_mapping()
        .ok_or_else(|| shape(context, "`with` must be a mapping"))?;
    mapping
        .iter()
        .map(|(key, item)| {
            let name = key
                .as_str()
                .ok_or_else(|| shape(context, "every `with` key must be a string"))?;
            let rendered = render_scalar(item)
                .ok_or_else(|| shape(context, &format!("input `{name}` must be a scalar")))?;
            Ok((name.to_owned(), rendered))
        })
        .collect()
}

/// Parses one step of a job.
///
/// # Errors
///
/// Returns an error when the step is not a mapping, has a mistyped field, or
/// neither runs a script nor uses an action.
fn parse_step(raw: &Value, context: &str) -> Result<Step, WorkflowError> {
    if raw.as_mapping().is_none() {
        return Err(shape(context, "every step must be a mapping"));
    }
    let step = Step {
        name: optional_string(raw, "name", context)?,
        uses: optional_string(raw, "uses", context)?,
        run: optional_string(raw, "run", context)?,
        with: parse_inputs(raw, context)?,
    };
    if step.uses.is_empty() && step.run.is_empty() {
        return Err(shape(context, "every step must set `uses` or `run`"));
    }
    Ok(step)
}

/// Parses one job of a workflow.
///
/// # Errors
///
/// Returns an error when a field is mistyped, `steps` is not a sequence, or
/// the job neither names a runner nor calls a reusable workflow.
fn parse_job(id: &str, raw: &Value, file: &str) -> Result<Job, WorkflowError> {
    let context = format!("{file}: job `{id}`");
    let steps = match raw.get("steps") {
        None => Vec::new(),
        Some(value) => value
            .as_sequence()
            .ok_or_else(|| shape(&context, "`steps` must be a sequence"))?
            .iter()
            .map(|step| parse_step(step, &context))
            .collect::<Result<Vec<Step>, WorkflowError>>()?,
    };
    let job = Job {
        id: id.to_owned(),
        runs_on: optional_string(raw, "runs-on", &context)?,
        uses: optional_string(raw, "uses", &context)?,
        timeout_minutes: optional_u64(raw, "timeout-minutes", &context)?,
        steps,
    };
    if job.runs_on.is_empty() && job.uses.is_empty() {
        return Err(shape(&context, "a job must set `runs-on` or `uses`"));
    }
    Ok(job)
}

/// Parses one workflow document.
///
/// # Errors
///
/// Returns an error when the text is not YAML, has no `jobs` mapping, or
/// contains a job or step of unexpected shape.
pub fn parse_workflow(file: &str, text: &str) -> Result<Workflow, WorkflowError> {
    let document: Value =
        serde_norway::from_str(text).map_err(|err| WorkflowError::Parse(file.to_owned(), err))?;
    let raw_jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or_else(|| shape(file, "missing a `jobs` mapping"))?;
    let jobs = raw_jobs
        .iter()
        .map(|(key, raw)| {
            let id = key
                .as_str()
                .ok_or_else(|| shape(file, "every job id must be a string"))?;
            parse_job(id, raw, file)
        })
        .collect::<Result<Vec<Job>, WorkflowError>>()?;
    Ok(Workflow {
        file: file.to_owned(),
        jobs,
    })
}

/// Lists the workflow file names inside an opened workflow directory.
///
/// # Errors
///
/// Returns an error when the directory cannot be listed or an entry's name
/// cannot be read.
fn workflow_names(dir: &Dir) -> Result<Vec<String>, WorkflowError> {
    let read = |err| WorkflowError::Read(WORKFLOW_DIR.to_owned(), err);
    let mut names: Vec<String> = Vec::new();
    for entry in dir.entries().map_err(read)? {
        let name = entry.map_err(read)?.file_name().map_err(read)?;
        let extension = Utf8Path::new(&name).extension().unwrap_or_default();
        if matches!(extension, "yml" | "yaml") {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// Loads and parses every workflow beneath `root`.
///
/// # Errors
///
/// Returns an error when the directory cannot be opened or listed, a file
/// cannot be read, or a file is not a workflow document.
pub fn load_workflows_in(root: &Utf8Path) -> Result<Vec<Workflow>, WorkflowError> {
    // The one ambient step: everything below reads through this capability,
    // which cannot escape the workflow directory.
    let dir = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    workflow_names(&dir)?
        .iter()
        .map(|name| {
            let text = dir
                .read_to_string(name)
                .map_err(|err| WorkflowError::Read(name.clone(), err))?;
            parse_workflow(name, &text)
        })
        .collect()
}

/// Loads and parses every workflow in this repository's `.github/workflows`.
///
/// # Errors
///
/// Returns the same errors as [`load_workflows_in`].
pub fn load_workflows() -> Result<Vec<Workflow>, WorkflowError> {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW_DIR);
    load_workflows_in(&root)
}

/// Reads a file from this repository's root through a directory capability.
///
/// # Errors
///
/// Returns an error when the repository root cannot be opened or the file
/// cannot be read.
pub fn read_repository_file(relative: &str) -> Result<String, WorkflowError> {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = Dir::open_ambient_dir(&root, ambient_authority())
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    dir.read_to_string(relative)
        .map_err(|err| WorkflowError::Read(relative.to_owned(), err))
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
