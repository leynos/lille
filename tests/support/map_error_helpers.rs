//! Helper functions for asserting on map errors in tests.
//!
//! This module is separate from `map_test_plugins` to avoid `dead_code` warnings
//! in test binaries that don't use error assertions. Only include this module
//! in tests that need to inspect captured map errors.

use bevy::prelude::*;
use lille::map::LilleMapError;

// Re-export CapturedMapErrors for convenience.
pub use super::map_test_plugins::CapturedMapErrors;

/// Returns errors captured by the map error capture observer.
///
/// Requires `install_map_error_capture` to have been called on the app.
pub fn captured_errors(app: &App) -> Vec<LilleMapError> {
    app.world()
        .get_resource::<CapturedMapErrors>()
        .map(|e| e.0.clone())
        .unwrap_or_default()
}
