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
/// The key is read as the string `on` and as the boolean true, since a YAML
/// 1.1 reader turns a bare `on` into the latter, and a document carrying both
/// has its triggers under both: a reader of one would miss the other's. The
/// shorthand forms are accepted too: `on: push` and `on: [push, ...]` mean the
/// same as the mapping.
///
/// # Errors
///
/// Returns an error when neither key is present or a value is not one of
/// those shapes.
pub fn parse_triggers(document: &Value, at: &Location) -> Result<Vec<String>, WorkflowError> {
    let declared: Vec<&Value> = [document.get("on"), document.get(Value::Bool(true))]
        .into_iter()
        .flatten()
        .collect();
    if declared.is_empty() {
        return Err(at.shape("missing an `on` trigger"));
    }
    let mut names = Vec::new();
    for raw in declared {
        names.extend(parse_trigger_value(raw, at)?);
    }
    Ok(names)
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
