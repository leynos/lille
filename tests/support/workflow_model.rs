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

/// A runner label, such as `ubuntu-latest` or `ubicloud-standard-2`.
///
/// Distinct from `ContextPath` because they are different domain values that
/// were both `String`: a guard and a label could be passed to each other's
/// functions, compared, and assigned, and nothing would notice. A runner
/// group's name is a third value again and deliberately stays a `String`: it
/// names a pool, not a runner.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunnerLabel(String);

/// A GitHub Actions context path, such as `github.event_name`.
///
/// The field a fork-fallback expression branches on. Never a runner label,
/// though both were `String` until a review asked why the model allowed them
/// to be interchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextPath(String);

macro_rules! string_newtype {
    ($name:ident, $what:literal) => {
        impl $name {
            #[doc = concat!("Wraps ", $what, ".")]
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[doc = concat!("Borrows the ", $what, " as text.")]
            ///
            /// The one boundary at which the wrapper is opened: a message, a
            /// comparison against a constant, or a set the contracts build.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_newtype!(RunnerLabel, "a runner label");
string_newtype!(ContextPath, "a context path");

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
    Labels(Vec<RunnerLabel>),
    /// A runner group, with the labels required within that group.
    Group {
        /// Name of the runner group, which is a pool rather than a runner.
        group: String,
        /// Labels required within the group, possibly empty.
        labels: Vec<RunnerLabel>,
    },
    /// Two runners, chosen between when the workflow is evaluated.
    ///
    /// A pull request from a fork cannot obtain an Ubicloud runner, so a lane
    /// that serves pull requests names a GitHub-hosted runner on one arm and
    /// this repository's own runner on the other. Both arms are runners the
    /// job may execute on, so both are labels for every rule that reads them.
    ForkFallback {
        /// The context field the expression branches on.
        guard: ContextPath,
        /// The fork's runner first, then this repository's own.
        arms: [RunnerLabel; 2],
    },
}

/// The runner labels GitHub hosts, written out rather than matched by prefix.
///
/// The registry question needs an exact set. A prefix test silently absorbs
/// any new label that looks hosted, so a lane moved onto an unknown image
/// would drop out of "in use" and its registration would go unnoticed. A name
/// added here is a deliberate statement that GitHub hosts it.
///
/// `is_hosted_label` keeps its prefix test for the separate question of
/// whether a whole job runs on GitHub's pool, where a new image of a known
/// family is a job that is still hosted.
pub const GITHUB_HOSTED_LABELS: [&str; 5] = [
    "ubuntu-latest",
    "ubuntu-24.04",
    "ubuntu-22.04",
    "windows-latest",
    "macos-latest",
];

/// Reports whether a label is one GitHub hosts, by name.
///
/// Membership of [`GITHUB_HOSTED_LABELS`], not a prefix test, which is the
/// whole point: `ubuntu-20.04` is a hosted family member this estate does not
/// use, so it must be reported rather than silently excused.
///
/// ```no_run
/// assert!(workflow_model::is_github_hosted_label("ubuntu-latest"));
/// assert!(workflow_model::is_github_hosted_label("macos-latest"));
/// assert!(!workflow_model::is_github_hosted_label("ubuntu-20.04"));
/// assert!(!workflow_model::is_github_hosted_label("ubicloud-standard-4"));
/// ```
#[must_use]
pub fn is_github_hosted_label(label: &str) -> bool {
    GITHUB_HOSTED_LABELS.contains(&label)
}

/// Reports whether a label names one of GitHub's own hosted images.
///
/// Read per label rather than per job, because a fork-fallback selection holds
/// one of each and the two questions asked about it differ: which arm must be
/// registered with actionlint, and whether the job as a whole is hosted.
#[must_use]
pub fn is_hosted_label(label: &str) -> bool {
    ["ubuntu-", "windows-", "macos-"]
        .iter()
        .any(|prefix| label.starts_with(prefix))
}

/// Reports whether a label names one of GitHub's own hosted Ubuntu images.
///
/// A narrower question than `is_hosted_label`, and a separate one. "Hosted"
/// answers who pays for the runner; this answers whether a job sits where the
/// placement rule puts the lanes that are not measured. The two were one
/// predicate, so a delayed-comment job moved to `windows-latest` satisfied a
/// contract whose message says `ubuntu-latest`, and the invariant the contract
/// exists for was no longer the one it read.
#[must_use]
pub fn is_hosted_ubuntu_label(label: &str) -> bool {
    label.starts_with("ubuntu-")
}

impl RunnerSelection {
    /// Returns the labels the selection requires, empty when it names none.
    #[must_use]
    pub fn labels(&self) -> &[RunnerLabel] {
        match self {
            Self::Delegated => &[],
            Self::Labels(labels) | Self::Group { labels, .. } => labels,
            Self::ForkFallback { arms, .. } => arms.as_slice(),
        }
    }

    /// Returns the runner this repository's own branches get, when there is one.
    ///
    /// A fork-fallback selection has two arms and only one of them is the
    /// measured shape, so a contract pinning that shape asks for this rather
    /// than for the whole label set.
    #[must_use]
    pub fn owned_label(&self) -> Option<&str> {
        match self {
            Self::Labels(labels) if labels.len() == 1 => labels.first().map(RunnerLabel::as_str),
            Self::ForkFallback { arms, .. } => arms.last().map(RunnerLabel::as_str),
            Self::Delegated | Self::Labels(_) | Self::Group { .. } => None,
        }
    }

    /// Returns the runner a fork's pull request gets, when the job names one.
    #[must_use]
    pub fn fork_label(&self) -> Option<&str> {
        match self {
            Self::ForkFallback { arms, .. } => arms.first().map(RunnerLabel::as_str),
            Self::Delegated | Self::Labels(_) | Self::Group { .. } => None,
        }
    }

    /// Returns the context field a fork-fallback selection branches on.
    #[must_use]
    pub fn guard(&self) -> Option<&str> {
        match self {
            Self::ForkFallback { guard, .. } => Some(guard.as_str()),
            Self::Delegated | Self::Labels(_) | Self::Group { .. } => None,
        }
    }

    /// Reads a scalar `runs-on` as a fork-fallback selection, or returns `None`.
    ///
    /// The grammar it is read against lives in `placement_expression`, so this
    /// type states the shapes a `runs-on` can have and that module states what
    /// counts as one of them.
    #[must_use]
    pub fn from_expression(text: &str) -> Option<Self> {
        // The grammar answers in plain strings because it is a reader of text.
        // The domain values are put on here, where the two are told apart.
        let (guard, [fork, owned]) = crate::placement_expression::read(text)?;
        Some(Self::ForkFallback {
            guard: ContextPath::new(guard),
            arms: [RunnerLabel::new(fork), RunnerLabel::new(owned)],
        })
    }

    /// Reports whether the job names a runner of its own.
    #[must_use]
    pub const fn names_a_runner(&self) -> bool {
        !matches!(self, Self::Delegated)
    }
}

/// Renders a label list the way a failure message should quote it.
fn render_labels(labels: &[RunnerLabel]) -> String {
    labels
        .iter()
        .map(RunnerLabel::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

impl fmt::Display for RunnerSelection {
    /// Renders the selection the way a failure message should quote it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Delegated => write!(f, "(reusable workflow)"),
            Self::Labels(labels) => write!(f, "{}", render_labels(labels)),
            Self::Group { group, labels } if labels.is_empty() => write!(f, "group {group}"),
            Self::Group { group, labels } => {
                write!(f, "group {group} ({})", render_labels(labels))
            }
            Self::ForkFallback { guard, arms } => {
                let [fork, owned] = arms;
                write!(f, "{fork} if {guard}, else {owned}")
            }
        }
    }
}

/// An `if` or `continue-on-error` value that is constantly false.
///
/// A workflow may write either as a bare YAML boolean or as an expression, so
/// the rendered scalar is compared against every spelling of a constant that
/// decides the same way every time the workflow runs.
const CONSTANT_FALSE: [&str; 3] = ["false", "${{ false }}", "${{false}}"];

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
