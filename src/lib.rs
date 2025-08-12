//! Library crate providing core Lille game logic.
//! Re-exports common components and systems for the main application and tests.
pub mod actor;
pub mod components;
pub mod constants;
pub mod dbsp_circuit;
pub mod dbsp_sync;
pub mod ddlog_sync;
pub mod entity;
pub mod logging;
pub mod physics;
pub mod spawn_world;
pub mod vector_math;
pub mod world_handle;
pub use constants::*;

// Re-export commonly used items
pub use actor::Actor;
pub use components::{DdlogId, ForceComp, Health, Target, UnitType, VelocityComp};
pub use dbsp_circuit::{
    DbspCircuit, FloorHeightAt, Force, HighestBlockAt, NewPosition, Position, PositionFloor,
};
pub use dbsp_circuit::{NewVelocity, Velocity};
pub use dbsp_sync::{
    apply_dbsp_outputs_system, cache_state_for_dbsp_system, init_dbsp_system, DbspPlugin,
};
pub use ddlog_sync::{apply_ddlog_deltas_system, cache_state_for_ddlog_system};
pub use entity::{BadGuy, Entity};
pub use logging::init as init_logging;
pub use physics::applied_acceleration;
pub use spawn_world::spawn_world_system;
pub use vector_math::{vec_mag, vec_normalize};
pub use world_handle::{init_world_handle_system, WorldHandle};

pub mod prelude {
    //! Prelude exports used in documentation examples.
    //!
    //! ```rust,no_run
    //! use lille::prelude::*;
    //! ```

    pub use crate::components::Block;
    pub use crate::DbspCircuit;
    pub use crate::DbspPlugin;
    pub use crate::FloorHeightAt;
    pub use crate::PositionFloor;
    pub use crate::Velocity;
    pub use ordered_float::OrderedFloat;
}
