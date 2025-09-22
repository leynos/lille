//! Input synchronisation systems bridging Bevy ECS into the DBSP circuit.

use std::{collections::HashSet, convert::TryFrom, mem};

use bevy::prelude::*;
use log::{debug, warn};

use crate::components::{
    Block, BlockSlope, DdlogId, ForceComp, Health, Target as TargetComp, VelocityComp,
};
use crate::dbsp_circuit::{
    DamageEvent, DbspCircuit, Force, HealthState, Position, Target, Velocity,
};
use crate::world_handle::{DdlogEntity, WorldHandle};
use dbsp::operator::input::ZSetHandle;

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
        let circuit = &state.circuit;
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
        sync_component(
            state,
            query,
            |row| Some(row.3.map(|v| (v.vx, v.vy, v.vz)).unwrap_or_default()),
            |entity, (vx, vy, vz)| Velocity {
                entity,
                vx: (vx as f64).into(),
                vy: (vy as f64).into(),
                vz: (vz as f64).into(),
            },
            |circuit: &DbspCircuit| circuit.velocity_in(),
        );
    }

    /// Synchronises entity targets with the DBSP circuit.
    fn sync_targets(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
        sync_component(
            state,
            query,
            |row| row.4.map(|t| (t.x, t.y)),
            |entity, (x, y)| Target {
                entity,
                x: (x as f64).into(),
                y: (y as f64).into(),
            },
            |circuit: &DbspCircuit| circuit.target_in(),
        );
    }

    /// Mirrors entity health into the circuit, enforcing clamps and logging once.
    fn sync_health(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
        let circuit = &state.circuit;
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

    /// Generic helper for syncing entity components to DBSP circuit inputs.
    fn sync_component<T, S, F, G, H>(
        state: &mut DbspState,
        query: &mut Query<EntityRow<'_>>,
        extract_component: F,
        create_struct: G,
        get_input_handle: H,
    ) where
        F: Fn(&EntityRow<'_>) -> Option<T>,
        G: Fn(i64, T) -> S,
        H: Fn(&DbspCircuit) -> &ZSetHandle<S>,
        S: Clone + dbsp::DBData,
    {
        let circuit = &state.circuit;
        for (entity, id, transform, velocity, target, mut health) in query.iter_mut() {
            let entity_key = id.0;
            let row_view = (
                entity,
                id,
                transform,
                velocity,
                target,
                health.as_deref_mut(),
            );
            if let Some(component_data) = extract_component(&row_view) {
                let input_struct = create_struct(entity_key, component_data);
                get_input_handle(circuit).push(input_struct, 1);
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
