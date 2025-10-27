//! Helper functions for caching ECS state into the DBSP circuit.

use std::convert::TryFrom;

use bevy::prelude::{Entity, Query};
use dbsp::operator::input::ZSetHandle;
use log::{debug, warn};

use crate::components::{
    Block, BlockSlope, DdlogId, ForceComp, Health, Target as TargetComp, VelocityComp,
};
use crate::dbsp_circuit::{DbspCircuit, Force, HealthState, Position, Target, Velocity};
use crate::world_handle::{DdlogEntity, WorldHandle};

use super::{DbspState, EntityRow, IdQueries};

/// Macro to generate component synchronisation wrapper functions.
macro_rules! sync_component_wrapper {
    (
        $(#[$meta:meta])* $fn_name:ident, $row_field:tt, $component_type:ty,
        $struct_name:ident { $($field:ident),+ }, $input_handle:ident
    ) => {
        $(#[$meta])*
        fn $fn_name(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
            sync_component(
                state,
                query,
                |row| row.$row_field.map(|component: &$component_type| {
                    ( $(component.$field),+ )
                }),
                |entity, ( $( $field ),+ )| $struct_name {
                    entity,
                    $( $field: f64::from($field).into() ),+
                },
                |circuit: &DbspCircuit| circuit.$input_handle(),
            );
        }
    };
}

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
                x: f64::from(transform.translation.x).into(),
                y: f64::from(transform.translation.y).into(),
                z: f64::from(transform.translation.z).into(),
            },
            1,
        );
        if world.entities.contains_key(&id.0) {
            // `cache_state_for_dbsp_system` clears the world handle before each
            // sync pass, so finding a pre-existing entry means stale state
            // leaked from a previous frame. Removing it keeps the cache
            // consistent without crashing release builds.
            warn!(
                "world handle entry for entity {} existed before sync; removing stale record",
                id.0
            );
            world.entities.remove(&id.0);
        }
    }
}

sync_component_wrapper!(
    /// Synchronises entity velocities with the DBSP circuit.
    sync_velocities,
    3,
    VelocityComp,
    Velocity { vx, vy, vz },
    velocity_in
);

sync_component_wrapper!(
    /// Synchronises entity targets with the DBSP circuit.
    sync_targets,
    4,
    TargetComp,
    Target { x, y },
    target_in
);

fn create_and_store_health_snapshot(state: &mut DbspState, entity: u64, current: u16, max: u16) {
    let circuit = &state.circuit;
    let snapshot = HealthState {
        entity,
        current,
        max,
    };
    circuit.health_state_in().push(snapshot, 1);
    state.health_snapshot.insert(entity, snapshot);
}

/// Mirrors entity health into the circuit, enforcing clamps and logging once.
fn sync_health(state: &mut DbspState, query: &mut Query<EntityRow<'_>>) {
    for (_, id, _, _, _, mut health_opt) in query.iter_mut() {
        let Some(health) = health_opt.as_deref_mut() else {
            continue;
        };
        let Ok(entity_id) = u64::try_from(id.0) else {
            warn!("health component for negative id {} skipped", id.0);
            continue;
        };
        let original_current = health.current;
        let (clamped_current, max_value, was_clamped) = clamp_health_values(health);
        if was_clamped {
            debug!(
                "health current {} clamped to {} for entity {}",
                original_current, clamped_current, id.0
            );
        }
        health.current = clamped_current;
        create_and_store_health_snapshot(state, entity_id, clamped_current, max_value);
    }
}

/// Generic helper for syncing entity components to DBSP circuit inputs.
#[expect(
    clippy::too_many_arguments,
    reason = "Helper coordinates multiple callback parameters for flexibility."
)]
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
    for (entity, id, transform, velocity, target, mut health_opt) in query.iter_mut() {
        let entity_key = id.0;
        let row_view = (
            entity,
            id,
            transform,
            velocity,
            target,
            health_opt.as_deref_mut(),
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
                    mass: f.mass.map(Into::into),
                },
                1,
            );
        } else {
            warn!("force component for unknown entity {entity:?} ignored");
        }
    }
}
