//! Synchronisation systems for integrating DBSP circuits with Bevy ECS.
//!
//! This module re-exports the plugin, state management, and synchronisation
//! systems that bridge Bevy ECS with the DBSP circuit.

mod damage_inbox;
mod duplicate_filter;
mod input;
mod output;
mod plugin;
mod state;

pub use damage_inbox::DamageInbox;
pub use input::{cache_state_for_dbsp_system, init_dbsp_system};
pub use output::apply_dbsp_outputs_system;
pub use plugin::{DbspPlugin, DbspSyncError, DbspSyncErrorContext};
pub use state::{DbspState, IdQueries};

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;
    use rstest::rstest;

    #[rstest]
    fn init_system_exposes_state() {
        let mut world = World::new();
        init_dbsp_system(&mut world).expect("failed to init DbspState");
        assert!(world.get_non_send_resource::<DbspState>().is_some());
    }

    #[rstest]
    fn damage_inbox_is_constructible() {
        let mut inbox = DamageInbox::default();
        inbox.push(crate::dbsp_circuit::DamageEvent {
            entity: 1,
            amount: 1,
            source: crate::dbsp_circuit::DamageSource::External,
            at_tick: 1,
            seq: None,
        });
        assert!(!inbox.is_empty());
    }

    #[rstest]
    fn plugin_is_default_constructible() {
        let _: DbspPlugin = DbspPlugin;
    }
}
