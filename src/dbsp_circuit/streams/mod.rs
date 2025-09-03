//! DBSP dataflow stream construction for spatial simulation.
//!
//! This module is split into domain-focused submodules and re-exports their
//! helpers for building the overall circuit.

pub(super) mod behaviour;
pub(super) mod floor;
pub(super) mod kinematics;

#[cfg(test)]
pub mod test_utils;

pub use behaviour::{apply_movement, fear_level_stream, movement_decision_stream};
pub use floor::{floor_height_stream, highest_block_pair};
pub use kinematics::{
    new_position_stream, new_velocity_stream, position_floor_stream, standing_motion_stream,
    PositionFloor,
};
