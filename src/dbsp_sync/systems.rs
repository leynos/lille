//! Systems bridging Bevy ECS with the DBSP circuit.

use std::{collections::HashSet, convert::TryFrom, mem};

use bevy::prelude::*;
use log::{debug, error, warn};

use crate::components::{
    Block, BlockSlope, DdlogId, ForceComp, Health, Target as TargetComp, VelocityComp,
};
use crate::dbsp_circuit::{
    try_step, DamageEvent, DbspCircuit, Force, HealthState, Position, Target, Velocity,
};
use crate::world_handle::{DdlogEntity, WorldHandle};

use super::{DamageInbox, DbspState, IdQueries};

type EntityRow<'w> = (
    Entity,
    &'w DdlogId,
    &'w Transform,
    Option<&'w VelocityComp>,
    Option<&'w TargetComp>,
    Option<&'w mut Health>,
);

/// Initializes the [`DbspState`] resource in the provided [`World`].
///
/// Call this once during Bevy startup before running any DBSP synchronisation
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
pub fn cache_state_for_dbsp_system(
    mut state: NonSendMut<DbspState>,
    mut entity_query: Query<EntityRow<'_>>,
    force_query: Query<(Entity, &DdlogId, &ForceComp)>,
    block_query: Query<(&Block, Option<&BlockSlope>)>,
    mut id_queries: IdQueries,
    mut damage_inbox: ResMut<DamageInbox>,
    mut world_handle: ResMut<WorldHandle>,
) {
    world_handle.blocks.clear();
    world_handle.slopes.clear();
    world_handle.entities.clear();

    let previous_snapshots = collect_previous_health_snapshots(&mut state);
    let pending_damage = mem::take(&mut state.pending_damage_retractions);
    state.expected_health_retractions.clear();

    sync::blocks(&mut state.circuit, &block_query, world_handle.as_mut());
    sync::id_maps(&mut state, &mut id_queries);
    sync::entities(&mut state, &mut entity_query, world_handle.as_mut());
    sync::forces(&mut state, &force_query);

    apply_health_snapshot_retractions(&mut state.circuit, &previous_snapshots);
    apply_damage_retractions(&mut state, pending_damage);

    ingest_damage_events(&mut state, damage_inbox.as_mut());
}

fn collect_previous_health_snapshots(state: &mut DbspState) -> Vec<HealthState> {
    let snapshots: Vec<_> = state.health_snapshot.values().copied().collect();
    state.health_snapshot.clear();
    snapshots
}

fn apply_health_snapshot_retractions(circuit: &mut DbspCircuit, snapshots: &[HealthState]) {
    for snapshot in snapshots {
        circuit.health_state_in().push(*snapshot, -1);
    }
}

fn apply_damage_retractions(state: &mut DbspState, retractions: Vec<DamageEvent>) {
    for event in retractions {
        state.circuit.damage_in().push(event, -1);
        state
            .expected_health_retractions
            .insert((event.entity, event.at_tick, event.seq));
    }
}

fn ingest_damage_events(state: &mut DbspState, inbox: &mut DamageInbox) {
    let mut sequenced_damage = HashSet::new();
    let mut unsequenced_damage = HashSet::new();
    for event in inbox.drain() {
        let duplicate = match event.seq {
            Some(_) => state.record_duplicate_sequenced_damage(&event, &mut sequenced_damage),
            None => state.record_duplicate_unsequenced_damage(&event, &mut unsequenced_damage),
        };
        if duplicate {
            continue;
        }
        state.circuit.damage_in().push(event, 1);
        state.pending_damage_retractions.push(event);
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
        state: &mut DbspState,
        query: &mut Query<EntityRow<'_>>,
        world: &mut WorldHandle,
    ) {
        sync_positions(state, query, world);
        sync_velocities(state, query);
        sync_targets(state, query);
        sync_health(state, query);
        update_world_handle(query, world);
    }

    /// Synchronises entity positions with the DBSP circuit.
    fn sync_positions(
        state: &mut DbspState,
        query: &mut Query<EntityRow<'_>>,
        world: &mut WorldHandle,
    ) {
        let circuit = &mut state.circuit;
        for (_, id, transform, _, _, _) in query.iter_mut() {
            circuit.position_in().push(
                Position {
                    entity: id.0,
                    x: (transform.translation.x as f64).into(),
                    y: (transform.translation.y as f64).into(),
                    z: (transform.translation.z as f64).into(),
                },
                1,
            );
            debug_assert!(world.entities.get(&id.0).is_none());
        }
    }

    /// Synchronises entity velocities with the DBSP circuit.
    fn sync_velocities(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
        let circuit = &mut state.circuit;
        for (_, id, _, vel, _, _) in query.iter_mut() {
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
        }
    }

    /// Synchronises entity targets with the DBSP circuit.
    fn sync_targets(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
        let circuit = &mut state.circuit;
        for (_, id, _, _, target, _) in query.iter_mut() {
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
        }
    }

    /// Mirrors entity health into the circuit, enforcing clamps and logging once.
    fn sync_health(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
        let circuit = &mut state.circuit;
        for (_, id, _, _, _, health) in query.iter_mut() {
            let Some(mut health) = health else {
                continue;
            };
            let original_current = health.current;
            let (clamped_current, max_value, was_clamped) = clamp_health_values(health.as_ref());
            if was_clamped {
                debug!(
                    "health current {} clamped to {} for entity {}",
                    original_current, clamped_current, id.0
                );
            }
            health.current = clamped_current;
            match u64::try_from(id.0) {
                Ok(entity_id) => {
                    let snapshot = HealthState {
                        entity: entity_id,
                        current: clamped_current,
                        max: max_value,
                    };
                    circuit.health_state_in().push(snapshot, 1);
                    state.health_snapshot.insert(entity_id, snapshot);
                }
                Err(_) => {
                    warn!("health component for negative id {} skipped", id.0);
                }
            }
        }
    }

    /// Rebuilds the cached world representation from the ECS query results.
    fn update_world_handle(query: &mut Query<EntityRow<'_>>, world: &mut WorldHandle) {
        for (_, id, transform, _, target, health) in query.iter_mut() {
            let entry = world
                .entities
                .entry(id.0)
                .or_insert_with(DdlogEntity::default);
            entry.position = transform.translation;
            entry.target = target.map(|t| t.0);
            if let Some(h) = health {
                let (clamped_current, max_value, _) = clamp_health_values(h.as_ref());
                entry.health_current = clamped_current;
                entry.health_max = max_value;
            } else {
                entry.health_current = 0;
                entry.health_max = 0;
            }
        }
    }

    fn clamp_health_values(health: &Health) -> (u16, u16, bool) {
        let clamped_current = health.current.min(health.max);
        (
            clamped_current,
            health.max,
            clamped_current != health.current,
        )
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
                if state.expected_health_retractions.remove(&(
                    delta.entity,
                    delta.at_tick,
                    delta.seq,
                )) {
                    continue;
                }
                if state.applied_health.get(&delta.entity) == Some(&key) {
                    debug!(
                        "duplicate health delta ignored for entity {} at tick {} seq {:?}",
                        delta.entity, delta.at_tick, delta.seq
                    );
                    state.health_duplicate_count += 1;
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

    state.expected_health_retractions.clear();
    state.circuit.clear_inputs();
}
