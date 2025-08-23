//! Synchronization systems for integrating DBSP circuits with Bevy ECS.
//!
//! This module provides a [`DbspPlugin`] that synchronises Bevy ECS state with
//! the [`DbspCircuit`]. The plugin inserts a [`DbspState`] resource, feeds
//! component data into the circuit each frame, steps the circuit, and applies
//! the results back to entities. The underlying systems are also exposed for
//! tests.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use log::{error, warn};

use crate::components::{
    Block, BlockSlope, DdlogId, ForceComp, Target as TargetComp, VelocityComp,
};
use crate::dbsp_circuit::{DbspCircuit, Force, Position, Target, Velocity};
use crate::world_handle::{DdlogEntity, WorldHandle};

// Compact alias for the per-entity inputs used by the cache system.
type EntityRow<'w> = (
    Entity,
    &'w DdlogId,
    &'w Transform, // Switch to &GlobalTransform if world-space is intended.
    Option<&'w VelocityComp>,
    Option<&'w TargetComp>,
);

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
    /// The map is maintained incrementally by
    /// [`cache_state_for_dbsp_system`] to avoid rebuilding it every frame.
    id_map: HashMap<i64, Entity>,
    /// Reverse mapping from Bevy [`Entity`] values to DBSP identifiers.
    rev_map: HashMap<Entity, i64>,
}

#[derive(SystemParam)]
pub struct IdQueries<'w, 's> {
    pub added: Query<'w, 's, (Entity, &'static DdlogId), Added<DdlogId>>,
    pub changed: Query<'w, 's, (Entity, &'static DdlogId), Changed<DdlogId>>,
    pub removed: RemovedComponents<'w, 's, DdlogId>,
}

impl DbspState {
    /// Creates a new [`DbspState`] with an initialized circuit.
    pub fn new() -> Result<Self, dbsp::Error> {
        Ok(Self {
            circuit: DbspCircuit::new()?,
            id_map: HashMap::new(),
            rev_map: HashMap::new(),
        })
    }

    /// Looks up the Bevy [`Entity`] for a DBSP identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lille::dbsp_sync::DbspState;
    /// let state = DbspState::new().expect("failed to initialise DbspState");
    /// assert!(state.entity_for_id(42).is_none());
    /// ```
    pub fn entity_for_id(&self, id: i64) -> Option<Entity> {
        self.id_map.get(&id).copied()
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
/// This system gathers `Transform`, optional `Velocity`, `Block`, and optional
/// `Force` components and pushes them into the circuit's input handles. Forces
/// for entities not present in the current position pass are ignored. It also
/// updates the internal mapping from DBSP entity identifiers to Bevy entities,
/// ensuring the lookup is maintained without rebuilding the map each frame.
/// When a [`WorldHandle`] resource is present, it is refreshed with the same
/// cached data for tests and diagnostics.
#[expect(
    clippy::type_complexity,
    reason = "Bevy Query uses tuple world refs; complexity hidden behind EntityRow alias"
)]
#[allow(unfulfilled_lint_expectations)]
pub fn cache_state_for_dbsp_system(
    mut state: NonSendMut<DbspState>,
    entity_query: Query<EntityRow<'_>>,
    force_query: Query<(Entity, &DdlogId, &ForceComp)>,
    block_query: Query<(&Block, Option<&BlockSlope>)>,
    mut id_queries: IdQueries,
    mut world_handle: Option<ResMut<WorldHandle>>,
) {
    if let Some(wh) = world_handle.as_mut() {
        // TODO: Only clear when inputs changed; otherwise keep prior snapshot.
        wh.blocks.clear();
        wh.slopes.clear();
        wh.entities.clear();
    }

    for (block, slope) in &block_query {
        state.circuit.block_in().push(block.clone(), 1);
        if let Some(s) = slope {
            state.circuit.block_slope_in().push(s.clone(), 1);
        }
        if let Some(ref mut wh) = world_handle {
            wh.blocks.push(block.clone());
            if let Some(s) = slope {
                wh.slopes.insert(s.block_id, s.clone());
            }
        }
    }

    // Remove mappings for entities whose `DdlogId` component was removed this
    // frame.
    for entity in id_queries.removed.read() {
        if let Some(old_id) = state.rev_map.remove(&entity) {
            state.id_map.remove(&old_id);
        }
    }

    // Replace mappings for entities whose identifier changed.
    for (entity, &DdlogId(new_id)) in &id_queries.changed {
        if let Some(old_id) = state.rev_map.insert(entity, new_id) {
            state.id_map.remove(&old_id);
        }
        if let Some(prev_entity) = state.id_map.insert(new_id, entity) {
            if prev_entity != entity {
                // Drop stale reverse mapping to maintain a bijection.
                state.rev_map.remove(&prev_entity);
                warn!("DdlogId {new_id} remapped from {prev_entity:?} to {entity:?}");
            }
        }
    }

    // Add mappings for newly spawned entities.
    for (entity, &DdlogId(id)) in &id_queries.added {
        if let Some(prev_entity) = state.id_map.insert(id, entity) {
            if prev_entity != entity {
                state.rev_map.remove(&prev_entity);
                warn!("DdlogId {id} remapped from {prev_entity:?} to {entity:?}");
            }
        }
        state.rev_map.insert(entity, id);
    }

    for (_, id, transform, vel, target) in &entity_query {
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

        if let Some(t) = target {
            state.circuit.target_in().push(
                Target {
                    entity: id.0,
                    x: (t.x as f64).into(),
                    y: (t.y as f64).into(),
                },
                1,
            );
        }

        if let Some(ref mut wh) = world_handle {
            wh.entities.insert(
                id.0,
                DdlogEntity {
                    position: transform.translation,
                    target: target.map(|t| t.0),
                    ..DdlogEntity::default()
                },
            );
        }
    }

    for (entity, id, f) in &force_query {
        // Only push forces for entities that already participated in the position
        // loop; this avoids introducing stray IDs.
        if state.id_map.contains_key(&id.0) {
            state.circuit.force_in().push(
                Force {
                    entity: id.0,
                    fx: f.force_x.into(),
                    fy: f.force_y.into(),
                    fz: f.force_z.into(),
                    mass: f.mass.map(|m| m.into()),
                },
                1,
            );
        } else {
            warn!("force component for unknown entity {entity:?} ignored");
        }
    }
}

/// Applies DBSP outputs back to ECS components.
///
/// Steps the circuit, reads new positions and velocities, and updates the
/// corresponding entities. When a [`WorldHandle`] resource is present, it is
/// updated with the latest positions for diagnostics. Output handles are
/// drained afterwards to avoid reapplying stale data.
pub fn apply_dbsp_outputs_system(
    mut state: NonSendMut<DbspState>,
    mut write_query: Query<(Entity, &mut Transform, Option<&mut VelocityComp>), With<DdlogId>>,
    mut world_handle: Option<ResMut<WorldHandle>>,
) {
    state.circuit.step().expect("DBSP step failed");

    let positions = state.circuit.new_position_out().consolidate();
    for (pos, _, _) in positions.iter() {
        if let Some(&entity) = state.id_map.get(&pos.entity) {
            if let Ok((_, mut transform, _)) = write_query.get_mut(entity) {
                transform.translation.x = pos.x.into_inner() as f32;
                transform.translation.y = pos.y.into_inner() as f32;
                transform.translation.z = pos.z.into_inner() as f32;
                if let Some(ref mut wh) = world_handle {
                    if let Some(e) = wh.entities.get_mut(&pos.entity) {
                        e.position = transform.translation;
                    }
                }
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
