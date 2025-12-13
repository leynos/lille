//! Bevy plugin wiring DBSP systems into the schedule.

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use log::error;
use thiserror::Error;

use crate::world_handle::init_world_handle_system;

use super::{
    apply_dbsp_outputs_system, cache_state_for_dbsp_system, init_dbsp_system, DamageInbox,
};

#[cfg(feature = "observers-v1-spike")]
use super::observers_v1;

/// Context carried by [`DbspSyncError`] events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbspSyncErrorContext {
    /// Failure surfaced while initialising the plugin.
    Init,
    /// Failure surfaced while advancing the DBSP circuit.
    Step,
}

/// Event raised when DBSP synchronisation hits an error path.
///
/// Observers log these events using Bevy's Events V2 pipeline so diagnostics
/// remain visible even when `bevy_log` is disabled.
#[derive(Event, Debug, Clone, Error)]
#[error("{context:?}: {detail}")]
pub struct DbspSyncError {
    /// Where the failure occurred.
    pub context: DbspSyncErrorContext,
    /// Description of the underlying error.
    pub detail: String,
}

impl DbspSyncError {
    /// Convenience constructor used by systems to emit error events.
    pub fn new(context: DbspSyncErrorContext, detail: impl Into<String>) -> Self {
        Self {
            context,
            detail: detail.into(),
        }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
fn log_dbsp_error(event: On<DbspSyncError>) {
    let DbspSyncError { context, detail } = event.event();
    error!("DBSP sync error during {context:?}: {detail}");
}

/// Bevy plugin installing systems that synchronise DBSP with the ECS world.
#[derive(Default)]
pub struct DbspPlugin;

impl Plugin for DbspPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(log_dbsp_error);

        #[cfg(feature = "observers-v1-spike")]
        app.add_observer(observers_v1::buffer_damage_ingress);

        if let Err(e) = init_dbsp_system(app.world_mut()) {
            app.world_mut().trigger(DbspSyncError::new(
                DbspSyncErrorContext::Init,
                e.to_string(),
            ));
            return;
        }

        app.init_resource::<DamageInbox>();
        app.add_systems(Startup, init_world_handle_system);

        #[cfg(feature = "observers-v1-spike")]
        app.add_systems(
            PostUpdate,
            (cache_state_for_dbsp_system, apply_dbsp_outputs_system).chain(),
        );

        #[cfg(not(feature = "observers-v1-spike"))]
        app.add_systems(
            Update,
            (cache_state_for_dbsp_system, apply_dbsp_outputs_system).chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbsp_sync::DbspState;
    use crate::world_handle::WorldHandle;
    use rstest::rstest;

    #[rstest]
    fn plugin_initialises_resources() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DbspPlugin);
        assert!(app.world().contains_resource::<DamageInbox>());
        assert!(app.world().get_non_send_resource::<DbspState>().is_some());
        app.update();
        assert!(app.world().contains_resource::<WorldHandle>());
    }
}
