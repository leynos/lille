//! Helpers for observing DBSP sync errors in integration tests.

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use lille::dbsp_sync::DbspSyncError;

/// Collected DBSP sync errors captured during tests.
/// Stored as plain context/detail strings to avoid cross-crate type mismatches.
#[derive(Resource, Default, Debug)]
pub struct CapturedErrors(pub Vec<(String, String)>);

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must take On<T> by value."
)]
fn record_error(event: On<DbspSyncError>, mut errors: ResMut<CapturedErrors>) {
    let err = event.event();
    errors
        .0
        .push((format!("{:?}", err.context), err.detail.clone()));
}

/// Installs the error-capturing observer and resource on the provided app.
pub fn install_error_observer(app: &mut App) {
    app.insert_resource(CapturedErrors::default());
    app.add_observer(record_error);
}
