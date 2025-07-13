//! Synchronization systems for integrating DBSP circuits with Bevy ECS.
//!
//! This module provides a [`DbspPlugin`] that synchronises Bevy ECS state with
//! the [`DbspCircuit`]. The plugin inserts a [`DbspState`] resource, feeds
//! component data into the circuit each frame, steps the circuit, and applies
//! the results back to entities. The underlying systems are also exposed for
//! tests.

use std::collections::HashMap;

use bevy::prelude::*;
use log::error;

use crate::components::{Block, DdlogId, VelocityComp};
use crate::dbsp_circuit::{DbspCircuit, Position, Velocity};

/// Bevy plugin that wires the DBSP circuit into the app.
///
/// Adding this plugin will insert [`DbspState`] as a non-send resource and
/// register the systems necessary to synchronise entity state with the DBSP
/// circuit on every frame.
#[derive(Default)]
pub struct DbspPlugin;

impl Plugin for DbspPlugin {
    fn build(&self, app: &mut App) {
        if let Err(e) = init_dbsp_system(&mut app.world) {
            error!("failed to init DBSP: {e}");
            return;
        }

        app.add_systems(
            Update,
            (cache_state_for_dbsp_system, apply_dbsp_outputs_system).chain(),
        );
    }
}

/// Non-send resource wrapping the [`DbspCircuit`].
///
/// [`DbspState`] owns the circuit instance that performs all physics and game
/// logic computations. It is inserted by [`DbspPlugin`] and persists outside the
/// ECS so the DBSP runtime can maintain state across frames. Systems use this
/// resource to push inputs and read outputs each tick.
pub struct DbspState {
    circuit: DbspCircuit,
    /// Cached mapping from DBSP entity IDs to Bevy `Entity` values.
    ///
    /// Rebuilding this map every frame can be costly, so it is updated when
    /// [`cache_state_for_dbsp_system`] runs.
    id_map: HashMap<i64, Entity>,
}

impl DbspState {
    /// Creates a new [`DbspState`] with an initialized circuit.
    pub fn new() -> Result<Self, dbsp::Error> {
        Ok(Self {
            circuit: DbspCircuit::new()?,
            id_map: HashMap::new(),
        })
    }
}

/// Initializes the [`DbspState`] resource in the provided [`World`].
///
/// Call this once during Bevy startup before running any DBSP synchronization
/// systems.
pub fn init_dbsp_system(world: &mut World) -> Result<(), dbsp::Error> {
    let state = DbspState::new()?;
    world.insert_non_send_resource(state);
    Ok(())
}

/// Caches current ECS state into the DBSP circuit inputs.
///
/// This system gathers `Transform`, optional `Velocity`, and `Block` components
/// and pushes them into the circuit's input handles.
pub fn cache_state_for_dbsp_system(
    mut state: NonSendMut<DbspState>,
    entity_query: Query<(Entity, &DdlogId, &Transform, Option<&VelocityComp>)>,
    block_query: Query<&Block>,
) {
    for block in &block_query {
        state.circuit.block_in().push(block.clone(), 1);
    }

    state.id_map.clear();
    for (entity, id, transform, vel) in &entity_query {
        state.id_map.insert(id.0, entity);
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
    mut write_query: Query<(Entity, &mut Transform, Option<&mut VelocityComp>), With<DdlogId>>,
) {
    state.circuit.step().expect("DBSP step failed");

    let positions = state.circuit.new_position_out().consolidate();
    for (pos, _, _) in positions.iter() {
        if let Some(&entity) = state.id_map.get(&pos.entity) {
            if let Ok((_, mut transform, _)) = write_query.get_mut(entity) {
                transform.translation.x = pos.x.into_inner() as f32;
                transform.translation.y = pos.y.into_inner() as f32;
                transform.translation.z = pos.z.into_inner() as f32;
            }
        }
    }

    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, _, _) in velocities.iter() {
        if let Some(&entity) = state.id_map.get(&vel.entity) {
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
