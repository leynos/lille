//! Library crate providing core Lille game logic.
//! Re-exports common components and systems for the main application and tests.
pub mod actor;
pub mod components;
pub mod ddlog_handle;
pub mod ddlog_sync;
pub mod entity;
pub mod logging;
pub mod spawn_world;
pub mod vector_math;
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

// Re-export commonly used items
pub use actor::Actor;
pub use components::{DdlogId, Health, Target, UnitType};
pub use ddlog_handle::STOP_CALLS;
pub use ddlog_handle::{init_ddlog_system, DdlogHandle};
pub use ddlog_sync::{apply_ddlog_deltas_system, cache_state_for_ddlog_system};
pub use entity::{BadGuy, Entity};
pub use logging::init as init_logging;
pub use spawn_world::spawn_world_system;
pub use vector_math::{vec_mag, vec_normalize};
