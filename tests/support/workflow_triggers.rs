//! Reads the event names a workflow declares under `on`.
//!
//! Split from the loader because `on` is the one field with two keys and three
//! shapes, and the reasons for accepting each are what the next reader needs.
//!
//! # Examples
//!
//! ```no_run
//! let document: serde_norway::Value = serde_norway::from_str("on: [push, pull_request]")?;
//! let at = workflow_estate::Location::file("scratch.yml");
//! assert_eq!(workflow_triggers::parse_triggers(&document, &at)?, ["push", "pull_request"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use serde_norway::Value;

use crate::workflow_estate::{Location, WorkflowError};
use crate::workflow_loader::render_scalar;

/// Reads the event names a workflow declares under `on`.
///
/// The key is read as the string `on` or as the boolean true, since a YAML
/// 1.1 reader turns a bare `on` into the latter. A document carrying both is
/// refused rather than merged: GitHub merges them, and a reader that picked
/// one would miss the other's triggers. The shorthand forms are accepted too:
/// `on: push` and `on: [push, ...]` mean the same as the mapping.
///
/// # Errors
///
/// Returns an error when neither key is present, both are, or a value is not
/// one of those shapes.
pub fn parse_triggers(document: &Value, at: &Location) -> Result<Vec<String>, WorkflowError> {
    match (document.get("on"), document.get(Value::Bool(true))) {
        (Some(raw), None) | (None, Some(raw)) => parse_trigger_value(raw, at),
        (None, None) => Err(at.shape("missing an `on` trigger")),
        // GitHub merges the two, so a reader of either is blind to the other's
        // triggers; a document spelling them twice has one spelling nobody meant.
        (Some(_), Some(_)) => {
            Err(at.shape("`on` is declared under both the string key and the boolean one"))
        }
    }
}

/// Reads the event names from one `on` value in any of its three shapes.
///
/// # Errors
///
/// Returns an error when the value is not an event, a list of events, or a
/// mapping keyed by event names.
fn parse_trigger_value(raw: &Value, at: &Location) -> Result<Vec<String>, WorkflowError> {
    if let Some(mapping) = raw.as_mapping() {
        return mapping
            .keys()
            .map(|key| {
                key.as_str()
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| at.shape("every `on` key must be a string"))
            })
            .collect();
    }
    if let Some(items) = raw.as_sequence() {
        return items
            .iter()
            .map(|item| {
                render_scalar(item).ok_or_else(|| at.shape("every `on` entry must be a scalar"))
            })
            .collect();
    }
    render_scalar(raw)
        .map(|event| vec![event])
        .ok_or_else(|| at.shape("`on` must be an event, a list of events, or a mapping"))
}
