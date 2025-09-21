//! Synchronization systems for integrating DBSP circuits with Bevy ECS.
//!
//! This module re-exports the plugin, state management, and synchronisation
//! systems that bridge Bevy ECS with the DBSP circuit.

mod damage_inbox;
mod plugin;
mod state;
mod systems;

pub use damage_inbox::DamageInbox;
pub use plugin::DbspPlugin;
pub use state::{DbspState, IdQueries};
pub use systems::{apply_dbsp_outputs_system, cache_state_for_dbsp_system, init_dbsp_system};
