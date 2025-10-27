#![cfg_attr(docsrs, feature(doc_cfg))]
//! Library crate providing core Lille game logic.
//! Re-exports common components and systems for the main application and tests.
pub mod actor;
pub mod components;
pub mod constants;
pub mod dbsp_circuit;
pub mod dbsp_sync;
pub mod entity;
pub mod logging;
mod macros;
pub mod numeric;
pub mod physics;
#[cfg(feature = "render")]
#[cfg_attr(docsrs, doc(cfg(feature = "render")))]
pub mod spawn_world;
pub mod vector_math;
pub mod world_handle;
pub use constants::*;

#[doc(hidden)]
pub mod __macro_deps {
    // Public solely for cross-crate macro hygiene; do not depend on it directly.
    // This module sits outside the semver-stable public API surface.
    pub use rkyv;
    pub use size_of;
}

// Re-export commonly used items
pub use actor::Actor;
pub use components::{DdlogId, ForceComp, Health, Target, UnitType, VelocityComp};
pub use dbsp_circuit::{
    DbspCircuit, FearLevel, FloorHeightAt, Force, HighestBlockAt, MovementDecision, NewPosition,
    Position, PositionFloor, Target as DbspTarget,
};
pub use dbsp_circuit::{NewVelocity, Velocity};
pub use dbsp_sync::{
    apply_dbsp_outputs_system, cache_state_for_dbsp_system, init_dbsp_system, DamageInbox,
    DbspPlugin,
};
pub use entity::{BadGuy, WorldEntity};
#[deprecated(
    since = "0.2.0",
    note = "Type renamed to `WorldEntity`. This alias will be removed in the next release."
)]
/// Legacy alias for [`WorldEntity`]; prefer the new name.
pub type Entity = WorldEntity;
pub use logging::init as init_logging;
pub use physics::{applied_acceleration, apply_ground_friction};
#[cfg(feature = "render")]
#[cfg_attr(docsrs, doc(cfg(feature = "render")))]
pub use spawn_world::spawn_world_system;
pub use vector_math::{vec_mag, vec_normalize};
pub use world_handle::{init_world_handle_system, WorldHandle};

pub mod prelude {
    //! Prelude exports used in documentation examples.
    //!
    //! ```rust,no_run
    //! use lille::prelude::*;
    //! ```

    pub use crate::applied_acceleration;
    pub use crate::components::Block;
    pub use crate::dbsp_circuit::Force;
    pub use crate::DbspCircuit;
    pub use crate::DbspPlugin;
    pub use crate::FloorHeightAt;
    pub use crate::PositionFloor;
    pub use crate::Velocity;
    pub use ordered_float::OrderedFloat;
}
