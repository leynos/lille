//! How a workflow job selects a runner, and the vocabulary of that choice.
//!
//! Split from `workflow_model`, which holds the shapes a job and a step have.
//! The seam is the question each answers: this module is about which machine
//! a lane asks for and who pays for it, and that question has its own domain
//! values, its own frozen set of GitHub's labels, and three predicates that
//! are easy to confuse with one another.
//!
//! # Examples
//!
//! ```no_run
//! assert!(runner_selection::is_github_hosted_label("ubuntu-latest"));
//! assert!(!runner_selection::is_github_hosted_label("ubicloud-standard-2"));
//! ```

use std::fmt;

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
/// assert!(runner_selection::is_github_hosted_label("ubuntu-latest"));
/// assert!(runner_selection::is_github_hosted_label("macos-latest"));
/// assert!(!runner_selection::is_github_hosted_label("ubuntu-20.04"));
/// assert!(!runner_selection::is_github_hosted_label("ubicloud-standard-4"));
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
