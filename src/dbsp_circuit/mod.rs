//! DBSP-based world inference engine.
//!
//! This module defines [`DbspCircuit`], the authoritative dataflow program for
//! Lille's game world. Callers feed [`Position`], [`Velocity`], [`Force`],
//! [`Target`], [`FearLevel`], and [`Block`](crate::components::Block) records into the circuit. Each tick
//! [`DbspCircuit::step`] derives movement decisions that yield updated
//! [`NewPosition`] and [`NewVelocity`] outputs alongside terrain queries like
//! [`HighestBlockAt`]. Input collections persist across steps—invoke
//! [`DbspCircuit::clear_inputs`] after each frame to prevent stale data from
//! affecting subsequent computations.

mod circuit;
mod helpers;
mod step;
pub mod streams;
pub mod types;

pub use circuit::DbspCircuit;
pub use step::{step, step_named, try_step};
pub use streams::{
    apply_movement, fall_damage_stream, fear_level_stream, floor_height_stream,
    health_delta_stream, highest_block_pair, movement_decision_stream, new_position_stream,
    new_velocity_stream, position_floor_stream, standing_motion_stream, PositionFloor,
};
pub use types::{
    DamageEvent, DamageSource, EntityId, FearLevel, FloorHeightAt, Force, HealthDelta, HealthState,
    HighestBlockAt, MovementDecision, NewPosition, NewVelocity, PlayerSpawnLocation, Position,
    SpawnPointRecord, Target, Tick, Velocity,
};

#[cfg(test)]
mod tests;
