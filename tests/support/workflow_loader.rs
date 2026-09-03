//! Reads and parses the repository's GitHub Actions workflow files.
//!
//! Parsing is strict about shape. A field that is present but of the wrong
//! type is an error rather than a silent default, because a contract that
//! read an empty string for a mistyped `runs-on` would pass a workflow it
//! should reject. Defaults are used only where a field is genuinely optional.
//!
//! Files are read through a `cap_std` directory capability rooted at the
//! directory being loaded, so the loader cannot reach outside it even if a
//! future contract passes it a name it should not.
//!
//! # Examples
//!
//! ```no_run
//! let workflows = workflow_loader::load_workflows()?;
//! assert!(workflows.iter().any(|w| w.file == "ci.yml"));
//! # Ok::<(), workflow_model::WorkflowError>(())
//! ```

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{ambient_authority, fs_utf8::Dir};
use serde_norway::Value;

use crate::workflow_model::{
    Job, Location, Step, Workflow, WorkflowError, WorkflowSource, WORKFLOW_DIR,
};

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
fn optional_string(raw: &Value, key: &str, at: &Location) -> Result<String, WorkflowError> {
    let Some(value) = raw.get(key) else {
        return Ok(String::new());
    };
    render_scalar(value).ok_or_else(|| at.shape(&format!("`{key}` must be a scalar")))
}

/// Reads an optional unsigned integer field.
///
/// # Errors
///
/// Returns an error when the field is present but is not an unsigned integer.
fn optional_u64(raw: &Value, key: &str, at: &Location) -> Result<Option<u64>, WorkflowError> {
    let Some(value) = raw.get(key) else {
        return Ok(None);
    };
    value
        .as_u64()
        .map(Some)
        .ok_or_else(|| at.shape(&format!("`{key}` must be an unsigned integer")))
}

/// Returns an error when `with` is not a mapping or an input is not a scalar.
fn parse_inputs(raw: &Value, at: &Location) -> Result<BTreeMap<String, String>, WorkflowError> {
    let Some(value) = raw.get("with") else {
        return Ok(BTreeMap::new());
    };
    let mapping = value
        .as_mapping()
        .ok_or_else(|| at.shape("`with` must be a mapping"))?;
    mapping
        .iter()
        .map(|(key, item)| {
            let name = key
                .as_str()
                .ok_or_else(|| at.shape("every `with` key must be a string"))?;
            let rendered = render_scalar(item)
                .ok_or_else(|| at.shape(&format!("input `{name}` must be a scalar")))?;
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
fn parse_step(raw: &Value, at: &Location) -> Result<Step, WorkflowError> {
    if raw.as_mapping().is_none() {
        return Err(at.shape("every step must be a mapping"));
    }
    let step = Step {
        name: optional_string(raw, "name", at)?,
        uses: optional_string(raw, "uses", at)?,
        run: optional_string(raw, "run", at)?,
        with: parse_inputs(raw, at)?,
    };
    if step.uses.is_empty() && step.run.is_empty() {
        return Err(at.shape("every step must set `uses` or `run`"));
    }
    Ok(step)
}

/// Parses one job of a workflow.
///
/// # Errors
///
/// Returns an error when a field is mistyped, `steps` is not a sequence, or
/// the job neither names a runner nor calls a reusable workflow.
fn parse_job(id: &str, raw: &Value, file: &Location) -> Result<Job, WorkflowError> {
    let at = file.job(id);
    let steps = match raw.get("steps") {
        None => Vec::new(),
        Some(value) => value
            .as_sequence()
            .ok_or_else(|| at.shape("`steps` must be a sequence"))?
            .iter()
            .map(|step| parse_step(step, &at))
            .collect::<Result<Vec<Step>, WorkflowError>>()?,
    };
    let job = Job {
        id: id.to_owned(),
        runs_on: optional_string(raw, "runs-on", &at)?,
        uses: optional_string(raw, "uses", &at)?,
        timeout_minutes: optional_u64(raw, "timeout-minutes", &at)?,
        steps,
    };
    if job.runs_on.is_empty() && job.uses.is_empty() {
        return Err(at.shape("a job must set `runs-on` or `uses`"));
    }
    Ok(job)
}

/// Parses one workflow document.
///
/// # Errors
///
/// Returns an error when the text is not YAML, has no `jobs` mapping, or
/// contains a job or step of unexpected shape.
pub fn parse_workflow(source: WorkflowSource<'_>) -> Result<Workflow, WorkflowError> {
    let at = Location::file(source.file);
    let document: Value = serde_norway::from_str(source.text)
        .map_err(|err| WorkflowError::Parse(source.file.to_owned(), err))?;
    let raw_jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or_else(|| at.shape("missing a `jobs` mapping"))?;
    let jobs = raw_jobs
        .iter()
        .map(|(key, raw)| {
            let id = key
                .as_str()
                .ok_or_else(|| at.shape("every job id must be a string"))?;
            parse_job(id, raw, &at)
        })
        .collect::<Result<Vec<Job>, WorkflowError>>()?;
    Ok(Workflow {
        file: source.file.to_owned(),
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
            parse_workflow(WorkflowSource {
                file: name,
                text: &text,
            })
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
