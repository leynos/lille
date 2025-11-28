//! Test-only helpers for observing DBSP sync errors.

#![cfg(test)]

use bevy::prelude::*;

use crate::dbsp_sync::DbspSyncError;

/// Collected DBSP sync errors captured during tests.
#[derive(Resource, Default, Debug)]
pub struct CapturedErrors(pub Vec<DbspSyncError>);

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must take On<T> by value."
)]
fn record_error(event: On<DbspSyncError>, mut errors: ResMut<CapturedErrors>) {
    errors.0.push(event.event().clone());
}

/// Installs the error-capturing observer and resource on the provided app.
pub fn install_error_observer(app: &mut App) {
    app.insert_resource(CapturedErrors::default());
    app.add_observer(record_error);
}
