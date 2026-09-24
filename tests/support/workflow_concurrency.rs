//! Parsing for a workflow's top-level `concurrency` block.
//!
//! It sits apart from `workflow_loader.rs` because the loader is already at
//! the module-size limit this repository lints for, and because the block
//! answers one question the rest of the loader does not ask: which runs may
//! cancel which.

use serde_norway::Value;

use crate::workflow_estate::{Concurrency, Location, WorkflowError};
use crate::workflow_loader::render_scalar;

/// Parses a workflow's top-level `concurrency` block.
///
/// The shorthand scalar form is accepted and recorded as a group with no
/// `cancel-in-progress`, which is what it means: reading it as "no
/// concurrency block" would let the shorthand escape every contract below.
///
/// # Errors
///
/// Returns an error when the block is neither a mapping nor a scalar, or
/// when either of its values is not a scalar.
pub fn parse_concurrency(
    document: &Value,
    at: &Location,
) -> Result<Option<Concurrency>, WorkflowError> {
    let Some(raw) = document.get("concurrency") else {
        return Ok(None);
    };
    if let Some(mapping) = raw.as_mapping() {
        let value = |key: &str| -> Result<String, WorkflowError> {
            mapping.get(Value::from(key)).map_or_else(
                || Ok(String::new()),
                |found| {
                    render_scalar(found)
                        .ok_or_else(|| at.shape("every `concurrency` value must be a scalar"))
                },
            )
        };
        return Ok(Some(Concurrency {
            group: value("group")?,
            cancel_in_progress: value("cancel-in-progress")?,
        }));
    }
    let group = render_scalar(raw)
        .ok_or_else(|| at.shape("`concurrency` must be a mapping or a scalar"))?;
    Ok(Some(Concurrency {
        group,
        cancel_in_progress: String::new(),
    }))
}
