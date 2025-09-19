//! Synchronization systems for integrating DBSP circuits with Bevy ECS.
//!
//! This module provides a [`DbspPlugin`] that synchronises Bevy ECS state with
//! the [`DbspCircuit`]. The plugin inserts a [`DbspState`] resource, feeds
//! component data into the circuit each frame, steps the circuit, and applies
//! the results back to entities. The underlying systems are also exposed for
//! tests.

use std::{collections::HashMap, convert::TryFrom};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use log::{error, warn};

use crate::components::{
    Block, BlockSlope, DdlogId, ForceComp, Health, Target as TargetComp, VelocityComp,
};
use crate::dbsp_circuit::{
    try_step, DamageEvent, DbspCircuit, EntityId, Force, HealthState, Position, Target, Tick,
    Velocity,
};
use crate::world_handle::{init_world_handle_system, DdlogEntity, WorldHandle};

// Compact alias for the per-entity inputs used by the cache system.
type EntityRow<'w> = (
    Entity,
    &'w DdlogId,
    &'w Transform,
    Option<&'w VelocityComp>,
    Option<&'w TargetComp>,
    Option<&'w Health>,
);

#[derive(Resource, Default)]
pub struct DamageInbox {
    events: Vec<DamageEvent>,
}

impl DamageInbox {
    pub fn push(&mut self, event: DamageEvent) {
        self.events.push(event);
    }

    pub fn extend<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = DamageEvent>,
    {
        self.events.extend(events);
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, DamageEvent> {
        self.events.drain(..)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

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

        app.init_resource::<DamageInbox>();
        app.add_systems(Startup, init_world_handle_system);
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
    applied_health: HashMap<EntityId, (Tick, Option<u32>)>,
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
            applied_health: HashMap::new(),
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
/// ensuring the lookup is maintained without rebuilding the map each frame. It
/// also refreshes the [`WorldHandle`] resource with the same cached data for
/// tests and diagnostics.
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
    mut damage_inbox: ResMut<DamageInbox>,
    mut world_handle: ResMut<WorldHandle>,
) {
    world_handle.blocks.clear();
    world_handle.slopes.clear();
    world_handle.entities.clear();

    sync::blocks(&mut state.circuit, &block_query, world_handle.as_mut());
    sync::id_maps(&mut state, &mut id_queries);
    sync::entities(&mut state.circuit, &entity_query, world_handle.as_mut());
    sync::forces(&mut state, &force_query);

    for event in damage_inbox.drain() {
        state.circuit.damage_in().push(event, 1);
    }
}

mod sync {
    use super::*;

    pub(super) fn blocks(
        circuit: &mut DbspCircuit,
        query: &Query<(&Block, Option<&BlockSlope>)>,
        world: &mut WorldHandle,
    ) {
        for (block, slope) in query.iter() {
            circuit.block_in().push(block.clone(), 1);
            if let Some(s) = slope {
                circuit.block_slope_in().push(s.clone(), 1);
            }
            world.blocks.push(block.clone());
            if let Some(s) = slope {
                world.slopes.insert(s.block_id, s.clone());
            }
        }
    }

    pub(super) fn id_maps(state: &mut DbspState, queries: &mut IdQueries) {
        for entity in queries.removed.read() {
            remove_entity_mapping(state, entity);
        }

        for (entity, &DdlogId(new_id)) in queries.changed.iter() {
            update_entity_mapping(state, entity, new_id);
        }

        for (entity, &DdlogId(id)) in queries.added.iter() {
            add_entity_mapping(state, entity, id);
        }
    }

    fn remove_entity_mapping(state: &mut DbspState, entity: Entity) {
        if let Some(old_id) = state.rev_map.remove(&entity) {
            state.id_map.remove(&old_id);
        }
    }

    fn update_entity_mapping(state: &mut DbspState, entity: Entity, new_id: i64) {
        if let Some(old_id) = state.rev_map.insert(entity, new_id) {
            state.id_map.remove(&old_id);
        }
        handle_id_conflict(state, entity, new_id);
    }

    fn add_entity_mapping(state: &mut DbspState, entity: Entity, id: i64) {
        handle_id_conflict(state, entity, id);
        state.rev_map.insert(entity, id);
    }

    /// Handles ID mapping conflicts by removing stale reverse mappings and
    /// logging warnings.
    fn handle_id_conflict(state: &mut DbspState, entity: Entity, id: i64) {
        if let Some(prev_entity) = state.id_map.insert(id, entity) {
            if prev_entity != entity {
                state.rev_map.remove(&prev_entity);
                warn!("DdlogId {id} remapped from {prev_entity:?} to {entity:?}");
            }
        }
    }

    pub(super) fn entities(
        circuit: &mut DbspCircuit,
        query: &Query<EntityRow<'_>>,
        world: &mut WorldHandle,
    ) {
        for (_, id, transform, vel, target, health) in query.iter() {
            circuit.position_in().push(
                Position {
                    entity: id.0,
                    x: (transform.translation.x as f64).into(),
                    y: (transform.translation.y as f64).into(),
                    z: (transform.translation.z as f64).into(),
                },
                1,
            );

            let v = vel.map(|v| (v.vx, v.vy, v.vz)).unwrap_or_default();
            circuit.velocity_in().push(
                Velocity {
                    entity: id.0,
                    vx: (v.0 as f64).into(),
                    vy: (v.1 as f64).into(),
                    vz: (v.2 as f64).into(),
                },
                1,
            );

            if let Some(t) = target {
                circuit.target_in().push(
                    Target {
                        entity: id.0,
                        x: (t.x as f64).into(),
                        y: (t.y as f64).into(),
                    },
                    1,
                );
            }

            let (health_current, health_max) = if let Some(h) = health {
                match u64::try_from(id.0) {
                    Ok(entity_id) => {
                        circuit.health_state_in().push(
                            HealthState {
                                entity: entity_id,
                                current: h.current,
                                max: h.max,
                            },
                            1,
                        );
                    }
                    Err(_) => {
                        warn!("health component for negative id {} skipped", id.0);
                    }
                }
                (h.current, h.max)
            } else {
                (0, 0)
            };

            world.entities.insert(
                id.0,
                DdlogEntity {
                    position: transform.translation,
                    target: target.map(|t| t.0),
                    health_current,
                    health_max,
                    ..DdlogEntity::default()
                },
            );
        }
    }

    pub(super) fn forces(state: &mut DbspState, query: &Query<(Entity, &DdlogId, &ForceComp)>) {
        for (entity, id, f) in query.iter() {
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
}

/// Applies DBSP outputs back to ECS components.
///
/// Steps the circuit, consolidates new positions and velocities, and updates
/// the corresponding entities. The [`WorldHandle`] resource is updated with the
/// latest positions for diagnostics.
///
/// Outputs are drained after application to prevent reapplying stale deltas on
/// subsequent frames.
#[expect(clippy::type_complexity, reason = "Bevy query tuples are idiomatic")]
pub fn apply_dbsp_outputs_system(
    mut state: NonSendMut<DbspState>,
    mut write_query: Query<
        (
            Entity,
            &mut Transform,
            Option<&mut VelocityComp>,
            Option<&mut Health>,
        ),
        With<DdlogId>,
    >,
    mut world_handle: ResMut<WorldHandle>,
) {
    if let Err(e) = try_step(&mut state.circuit) {
        error!("DbspCircuit::step failed: {e}");
        return;
    }

    let positions = state.circuit.new_position_out().consolidate();
    for (pos, _, _) in positions.iter() {
        if let Some(&entity) = state.id_map.get(&pos.entity) {
            if let Ok((_, mut transform, _, _)) = write_query.get_mut(entity) {
                transform.translation.x = pos.x.into_inner() as f32;
                transform.translation.y = pos.y.into_inner() as f32;
                transform.translation.z = pos.z.into_inner() as f32;
                if let Some(e) = world_handle.entities.get_mut(&pos.entity) {
                    e.position = transform.translation;
                }
            }
        }
    }

    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, _, _) in velocities.iter() {
        if let Some(&entity) = state.id_map.get(&vel.entity) {
            if let Ok((_, _, Some(mut velocity), _)) = write_query.get_mut(entity) {
                velocity.vx = vel.vx.into_inner() as f32;
                velocity.vy = vel.vy.into_inner() as f32;
                velocity.vz = vel.vz.into_inner() as f32;
            }
        }
    }

    let health_deltas = state.circuit.health_delta_out().consolidate();
    for (delta, _, _) in health_deltas.iter() {
        let Ok(entity_key) = i64::try_from(delta.entity) else {
            warn!("health delta for unmappable entity {}", delta.entity);
            continue;
        };
        let Some(&entity) = state.id_map.get(&entity_key) else {
            warn!("health delta for unknown entity id {}", delta.entity);
            continue;
        };
        if let Ok((_, _, _, maybe_health)) = write_query.get_mut(entity) {
            if let Some(mut health) = maybe_health {
                let key = (delta.at_tick, delta.seq);
                if state.applied_health.get(&delta.entity) == Some(&key) {
                    continue;
                }
                let current = i32::from(health.current);
                let max = i32::from(health.max);
                let new_value = (current + delta.delta).clamp(0, max);
                health.current = new_value as u16;
                state.applied_health.insert(delta.entity, key);
                if let Some(entry) = world_handle.entities.get_mut(&entity_key) {
                    entry.health_current = health.current;
                    entry.health_max = health.max;
                }
                if delta.death {
                    // Future hook: notify AI about deaths if needed.
                }
            } else {
                warn!("health delta received for entity without Health component");
            }
        }
    }
    let _ = state.circuit.health_delta_out().take_from_all();

    // Drain any remaining output so stale values are not reused.
    let _ = state.circuit.new_position_out().take_from_all();
    let _ = state.circuit.new_velocity_out().take_from_all();

    state.circuit.clear_inputs();
}
