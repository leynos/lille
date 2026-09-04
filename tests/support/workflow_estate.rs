//! Loading-facing and contract-facing workflow support.
//!
//! The estate's pinned commits and runner labels, the errors and locations
//! parsing reports, and the whole-file `Workflow` type. `workflow_model.rs`
//! holds the job and step shapes these are built from, which the property
//! tests share.
//!
//! # Examples
//!
//! ```no_run
//! let at = workflow_estate::Location::file("ci.yml");
//! assert!(at.shape("bad").to_string().contains("ci.yml"));
//! ```

use std::fmt;

use crate::workflow_model::Job;

/// Directory holding the repository's workflow definitions.
pub const WORKFLOW_DIR: &str = ".github/workflows";

/// Commit that every `actions/cache` reference must pin (v6.1.0).
pub const CACHE_ACTION_SHA: &str = "55cc8345863c7cc4c66a329aec7e433d2d1c52a9";

/// Commit that every `leynos/shared-actions` reference must pin.
pub const SHARED_ACTIONS_SHA: &str = "c6125f19593668cbfefd65a59c08cb7aefe90d93";

/// Runner label used by this repository's Ubicloud build and test jobs.
pub const UBICLOUD_LABEL: &str = "ubicloud-standard-8";

/// Publisher whose composite actions this repository is allowed to call.
pub const SHARED_ACTIONS_OWNER: &str = "leynos/shared-actions";

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

/// One workflow file.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// File name within [`WORKFLOW_DIR`].
    pub file: String,
    /// Event names under `on`, in declaration order.
    pub triggers: Vec<String>,
    /// Jobs in declaration order.
    pub jobs: Vec<Job>,
}

impl Workflow {
    /// Reports whether the workflow declares the named trigger.
    #[must_use]
    pub fn has_trigger(&self, event: &str) -> bool {
        self.triggers.iter().any(|name| name == event)
    }
}
