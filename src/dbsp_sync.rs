//! Synchronization systems for integrating DBSP circuits with Bevy ECS.
//!
//! This module provides Bevy systems that feed ECS component state into the
//! [`DbspCircuit`], step the circuit, and apply the resulting updates back to
//! entities. Use [`init_dbsp_system`] once during startup, then run
//! [`cache_state_for_dbsp_system`] and [`apply_dbsp_outputs_system`] each tick.

use std::collections::HashMap;

use bevy::prelude::*;
use log::error;

use crate::components::{Block, DdlogId, Velocity as VelocityComp};
use crate::dbsp_circuit::{DbspCircuit, Position, Velocity};

/// Non-send resource wrapping the [`DbspCircuit`].
///
/// This resource owns the circuit instance that performs all physics and game
/// logic computations. It must live outside the ECS so that the DBSP runtime can
/// maintain internal state across frames.
pub struct DbspState {
    circuit: DbspCircuit,
}

impl Default for DbspState {
    fn default() -> Self {
        Self::new()
    }
}

impl DbspState {
    /// Creates a new [`DbspState`] with an initialized circuit.
    ///
    /// # Panics
    ///
    /// Panics if the DBSP circuit cannot be constructed.
    pub fn new() -> Self {
        Self {
            circuit: DbspCircuit::new().expect("failed to build DBSP circuit"),
        }
    }
}

/// Initializes the [`DbspState`] resource in the provided [`World`].
///
/// Call this once during Bevy startup before running any DBSP synchronization
/// systems.
pub fn init_dbsp_system(world: &mut World) {
    world.insert_non_send_resource(DbspState::default());
}

/// Caches current ECS state into the DBSP circuit inputs.
///
/// This system gathers `Transform`, optional `Velocity`, and `Block` components
/// and pushes them into the circuit's input handles.
pub fn cache_state_for_dbsp_system(
    state: NonSendMut<DbspState>,
    entity_query: Query<(&DdlogId, &Transform, Option<&VelocityComp>)>,
    block_query: Query<&Block>,
) {
    for block in &block_query {
        state.circuit.block_in().push(block.clone(), 1);
    }

    for (id, transform, vel) in &entity_query {
        state.circuit.position_in().push(
            Position {
                entity: id.0,
                x: (transform.translation.x as f64).into(),
                y: (transform.translation.y as f64).into(),
                z: (transform.translation.z as f64).into(),
            },
            1,
        );

        let v = vel.map(|v| (v.vx, v.vy, v.vz)).unwrap_or_default();
        state.circuit.velocity_in().push(
            Velocity {
                entity: id.0,
                vx: (v.0 as f64).into(),
                vy: (v.1 as f64).into(),
                vz: (v.2 as f64).into(),
            },
            1,
        );
    }
}

/// Applies DBSP outputs back to ECS components.
///
/// Steps the circuit, reads new positions and velocities, and updates the
/// corresponding entities. Output handles are drained afterwards to avoid
/// reapplying stale data.
pub fn apply_dbsp_outputs_system(
    mut state: NonSendMut<DbspState>,
    id_query: Query<(Entity, &DdlogId)>,
    mut write_query: Query<(Entity, &mut Transform, Option<&mut VelocityComp>), With<DdlogId>>,
) {
    if let Err(e) = state.circuit.step() {
        error!("DBSP step failed: {e}");
        return;
    }

    let id_map: HashMap<i64, Entity> = id_query.iter().map(|(e, id)| (id.0, e)).collect();

    let positions = state.circuit.new_position_out().consolidate();
    for (pos, _, _) in positions.iter() {
        if let Some(&entity) = id_map.get(&pos.entity) {
            if let Ok((_, mut transform, _)) = write_query.get_mut(entity) {
                transform.translation.x = pos.x.into_inner() as f32;
                transform.translation.y = pos.y.into_inner() as f32;
                transform.translation.z = pos.z.into_inner() as f32;
            }
        }
    }

    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, _, _) in velocities.iter() {
        if let Some(&entity) = id_map.get(&vel.entity) {
            if let Ok((_, _, Some(mut v))) = write_query.get_mut(entity) {
                v.vx = vel.vx.into_inner() as f32;
                v.vy = vel.vy.into_inner() as f32;
                v.vz = vel.vz.into_inner() as f32;
            }
        }
    }

    // Drain any remaining output so stale values are not reused.
    let _ = state.circuit.new_position_out().take_from_all();
    let _ = state.circuit.new_velocity_out().take_from_all();

    state.circuit.clear_inputs();
}
