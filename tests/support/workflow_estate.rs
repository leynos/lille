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

/// Commit that a `leynos/shared-actions` reference must pin by default.
pub const SHARED_ACTIONS_SHA: &str = "c5a54701c8603a0fa756a6b34c49bc2af75a6c11";

/// Commit the `CodeScene` uploader must pin.
///
/// The uploader moved ahead of the rest of the estate on its own schedule.
/// At this revision its committed `cli-manifest.json` is the trust anchor
/// for the `cs-coverage` archive, and the action rejects the deprecated
/// `installer-checksum` input outright, so a caller that still passes it
/// fails rather than silently ignoring a checksum nobody checks.
pub const UPLOAD_CODESCENE_COVERAGE_SHA: &str = "a5765019912a8ab6882b12db049c7cde635f3a85";

/// Shared actions whose reviewed revision is not [`SHARED_ACTIONS_SHA`].
///
/// The estate's rule is that no reference floats, not that every action
/// moves together. Holding them all to one constant made the rule easy to
/// state and impossible to satisfy the moment one action had to move alone,
/// which is how a deliberate repin turns into a repository-wide repin
/// nobody asked for. Each exception is named here with the action it
/// governs, so a reference is still held to a reviewed commit by value and
/// a new exception has to be added deliberately.
pub const SHARED_ACTION_PIN_EXCEPTIONS: [(&str, &str); 1] =
    [("upload-codescene-coverage", UPLOAD_CODESCENE_COVERAGE_SHA)];

/// Return the commit a `leynos/shared-actions` reference must pin.
///
/// `uses` is the whole `owner/repo/.github/actions/<name>@<ref>` coordinate.
/// An action with no exception takes [`SHARED_ACTIONS_SHA`].
#[must_use]
pub fn required_shared_action_sha(uses: &str) -> &'static str {
    let path = uses.split('@').next().unwrap_or(uses);
    let name = path.rsplit('/').next().unwrap_or(path);
    SHARED_ACTION_PIN_EXCEPTIONS
        .iter()
        .find(|(action, _)| *action == name)
        .map_or(SHARED_ACTIONS_SHA, |(_, sha)| *sha)
}

/// Runner label used by this repository's Ubicloud build and test jobs.
pub const UBICLOUD_LABEL: &str = "ubicloud-standard-4";

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

/// A workflow's top-level `concurrency` block.
///
/// Both fields are rendered as written rather than interpreted. A literal
/// `cancel-in-progress: true` and the expression that conditions
/// cancellation on the event are both valid YAML for the same key, and a
/// contract that read the literal as a boolean could not tell them apart.
#[derive(Debug, Clone, Default)]
pub struct Concurrency {
    /// The `group` expression, empty when the block declares none.
    pub group: String,
    /// The `cancel-in-progress` value, empty when the block declares none.
    pub cancel_in_progress: String,
}

/// One workflow file.
#[derive(Debug, Clone)]
pub struct Workflow {
    /// File name within [`WORKFLOW_DIR`].
    pub file: String,
    /// Event names under `on`, in declaration order.
    pub triggers: Vec<String>,
    /// The top-level `concurrency` block, absent when none is declared.
    pub concurrency: Option<Concurrency>,
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
