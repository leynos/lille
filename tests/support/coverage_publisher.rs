//! What the one workflow allowed to publish coverage must itself hold to.
//!
//! `coverage-main.yml` owns the `CodeScene` upload and the ratchet baseline
//! every pull request compares against. Moving the upload there is only a
//! boundary if the publisher cannot be made to publish from anywhere else and
//! cannot be cancelled halfway through writing what the next pull request
//! will read.
//!
//! Here the trunk guard is the trigger itself: the publisher answers a push to
//! `main` and nothing else, which the contract asserts by equality. A
//! condition-level guard, and the `&&`/`||` reading it would need, has nothing
//! to guard against while no other trigger can start the workflow; adding a
//! dispatch trigger fails the equality first.
//!
//! These read the raw document, since the typed model carries neither the push
//! filter nor the concurrency settings.
//!
//! # Examples
//!
//! ```no_run
//! let document: serde_norway::Value =
//!     serde_norway::from_str("on:\n  push:\n    branches: [main]\njobs: {}\n")?;
//! assert_eq!(coverage_publisher::push_branches(&document), Some(vec!["main".to_owned()]));
//! # Ok::<(), serde_norway::Error>(())
//! ```

use serde_norway::Value;

/// The branch the publisher's push trigger must name, and name alone.
pub const TRUNK_BRANCH: &str = "main";

/// Returns the `on` values of a document under both keys it can arrive under.
fn trigger_values(document: &Value) -> impl Iterator<Item = &Value> {
    [document.get("on"), document.get(Value::Bool(true))]
        .into_iter()
        .flatten()
}

/// Returns the branch filter on a document's push trigger, if it declares one.
#[must_use]
pub fn push_branches(document: &Value) -> Option<Vec<String>> {
    trigger_values(document)
        .filter_map(|declared| declared.get("push")?.get("branches")?.as_sequence())
        .map(|branches| {
            branches
                .iter()
                .map(|branch| branch.as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .next()
}

/// Reports whether one concurrency setting can cancel a run in progress.
///
/// A group given as a bare string never cancels. Anything other than an
/// absent or false `cancel-in-progress` is refused, an expression included,
/// because an expression that reads false today is one edit from true.
fn cancels(concurrency: Option<&Value>) -> bool {
    let Some(setting) = concurrency.and_then(|value| value.get("cancel-in-progress")) else {
        return false;
    };
    !matches!(setting, Value::Bool(false)) && setting.as_str() != Some("false")
}

/// Returns every scope in a document whose concurrency cancels in progress.
#[must_use]
pub fn cancelling_scopes(document: &Value) -> Vec<String> {
    let mut scopes = Vec::new();
    if cancels(document.get("concurrency")) {
        scopes.push("the workflow".to_owned());
    }
    let jobs = document.get("jobs").and_then(Value::as_mapping);
    for (id, job) in jobs.into_iter().flatten() {
        if cancels(job.get("concurrency")) {
            scopes.push(format!("job {}", id.as_str().unwrap_or_default()));
        }
    }
    scopes
}

/// The credential binding and the input one upload step declares.
#[derive(Debug, PartialEq, Eq)]
pub struct TokenBinding {
    /// The step's `env.CS_ACCESS_TOKEN`, if it binds one.
    pub env: Option<String>,
    /// The step's `with.access-token`, if it passes one.
    pub input: Option<String>,
}

/// Returns the binding every step calling `action` declares.
///
/// Read positively because the step's non-empty guard passes with the binding
/// deleted, and the upload then skips on every run without failing anything.
#[must_use]
pub fn token_bindings(document: &Value, action: &str) -> Vec<TokenBinding> {
    let text = |value: Option<&Value>| value.and_then(Value::as_str).map(ToOwned::to_owned);
    document
        .get("jobs")
        .and_then(Value::as_mapping)
        .into_iter()
        .flat_map(|jobs| jobs.values())
        .filter_map(|job| job.get("steps").and_then(Value::as_sequence))
        .flatten()
        .filter(|step| {
            step.get("uses")
                .and_then(Value::as_str)
                .is_some_and(|uses| uses.split('@').next() == Some(action))
        })
        .map(|step| TokenBinding {
            env: text(step.get("env").and_then(|env| env.get("CS_ACCESS_TOKEN"))),
            input: text(step.get("with").and_then(|with| with.get("access-token"))),
        })
        .collect()
}
