//! Repository configuration this estate's contracts read.
//!
//! `workflow_loader.rs` reads `.github/workflows`. This module reads the other
//! repository files a contract needs, currently only `actionlint`'s runner
//! registration. Kept apart because the subject differs: a failure here is a
//! configuration file the contracts could not read, not a workflow they could
//! not parse.
//!
//! Files are read through a `cap_std` directory capability rooted at the
//! repository, so this module cannot reach outside it.
//!
//! # Examples
//!
//! ```no_run
//! let labels = workflow_config::registered_runner_labels()?;
//! assert!(labels.iter().all(|label| !label.is_empty()));
//! # Ok::<(), workflow_estate::WorkflowError>(())
//! ```

use camino::Utf8PathBuf;
use cap_std::{ambient_authority, fs_utf8::Dir};
use serde_norway::Value;

use crate::workflow_estate::{Location, WorkflowError};
use crate::workflow_loader::render_scalar;

/// Reads a file from this repository's root through a directory capability.
///
/// # Errors
///
/// Returns an error when the repository root cannot be opened or the file
/// cannot be read.
fn read_repository_file(relative: &str) -> Result<String, WorkflowError> {
    let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = Dir::open_ambient_dir(&root, ambient_authority())
        .map_err(|err| WorkflowError::Read(root.to_string(), err))?;
    dir.read_to_string(relative)
        .map_err(|err| WorkflowError::Read(relative.to_owned(), err))
}

/// Reads the self-hosted runner labels `actionlint` is configured to accept.
///
/// Parsed rather than searched as text: a substring test would accept
/// `standard-8` because `ubicloud-standard-8` contains it, and would accept a
/// label that appears only in a comment. The contract exists to prove a label
/// is registered, so it has to compare whole entries.
///
/// # Errors
///
/// Returns an error when the file cannot be read or is not a mapping whose
/// `self-hosted-runner.labels` is a sequence of strings.
pub fn registered_runner_labels() -> Result<Vec<String>, WorkflowError> {
    const FILE: &str = ".github/actionlint.yaml";
    let at = Location::file(FILE);
    let text = read_repository_file(FILE)?;
    let document: Value =
        serde_norway::from_str(&text).map_err(|err| WorkflowError::Parse(FILE.to_owned(), err))?;
    let Some(labels) = document
        .get("self-hosted-runner")
        .and_then(|it| it.get("labels"))
    else {
        return Ok(Vec::new());
    };
    labels
        .as_sequence()
        .ok_or_else(|| at.shape("`self-hosted-runner.labels` must be a sequence"))?
        .iter()
        .map(|label| {
            render_scalar(label).ok_or_else(|| at.shape("every registered label must be a scalar"))
        })
        .collect()
}
