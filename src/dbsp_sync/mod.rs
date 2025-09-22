//! Synchronization systems for integrating DBSP circuits with Bevy ECS.
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
pub use plugin::DbspPlugin;
pub use state::{DbspState, IdQueries};
