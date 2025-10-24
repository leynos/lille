//! Bevy plugin wiring DBSP systems into the schedule.

use bevy::prelude::*;
use log::error;

use crate::world_handle::init_world_handle_system;

use super::{
    apply_dbsp_outputs_system, cache_state_for_dbsp_system, init_dbsp_system, DamageInbox,
};

#[derive(Default)]
/// Bevy plugin installing systems that synchronise DBSP with the ECS world.
pub struct DbspPlugin;

impl Plugin for DbspPlugin {
    fn build(&self, app: &mut App) {
        if let Err(e) = init_dbsp_system(&mut app.world) {
            error!("failed to init DBSP: {e}");
            return;
        }

        app.init_resource::<DamageInbox>();
        app.add_systems(Startup, init_world_handle_system);
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
        assert!(app.world.contains_resource::<DamageInbox>());
        assert!(app.world.get_non_send_resource::<DbspState>().is_some());
        app.update();
        assert!(app.world.contains_resource::<WorldHandle>());
    }
}
