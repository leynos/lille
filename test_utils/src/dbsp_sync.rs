//! Helpers for observing DBSP sync errors in tests.
use bevy::ecs::prelude::On;
use bevy::prelude::*;
use lille::dbsp_sync::DbspSyncError;

/// Collected DBSP sync errors captured during tests.
/// Stored as `(context, detail)` strings to avoid cross-crate type mismatches.
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
    app.world_mut().add_observer(record_error);
}

#[cfg(test)]
mod tests {
    use super::*;
    use lille::dbsp_sync::DbspSyncErrorContext;

    #[test]
    fn observer_records_triggered_error() {
        let mut app = App::new();
        install_error_observer(&mut app);
        app.world_mut()
            .trigger(DbspSyncError::new(DbspSyncErrorContext::Step, "probe"));
        assert!(
            !app.world().resource::<CapturedErrors>().0.is_empty(),
            "observer should record triggered error"
        );
    }
}
