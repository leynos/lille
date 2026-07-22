//! Output application systems bridging DBSP updates back into Bevy ECS.

use std::convert::TryFrom;

use bevy::prelude::*;
use log::{debug, warn};

use crate::components::{DdlogId, Health, VelocityComp};
use crate::dbsp_circuit::{HealthDelta, Position, Tick};
use crate::world_handle::WorldHandle;

use super::{DbspState, DbspSyncError, DbspSyncErrorContext};

type DbspWriteQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        Option<&'static mut VelocityComp>,
        Option<&'static mut Health>,
    ),
    With<DdlogId>,
>;

/// Mutable component handles for an entity resolved through the DBSP id map.
type WriteTarget<'q> = (
    Mut<'q, Transform>,
    Option<Mut<'q, VelocityComp>>,
    Option<Mut<'q, Health>>,
);

/// Resolves a DBSP record's entity through `state.id_map` and borrows its
/// mutable `Transform`/`VelocityComp`/`Health` components from the query.
///
/// Returns `None` when the id is unmapped or the entity is missing from the
/// query, mirroring the skip behaviour of the previous macro without binding a
/// pattern fragment.
fn resolve_write_target<'q>(
    state: &DbspState,
    write_query: &'q mut DbspWriteQuery<'_, '_>,
    entity_id: i64,
) -> Option<WriteTarget<'q>> {
    let &entity = state.id_map.get(&entity_id)?;
    let (_, transform, velocity, health) = write_query.get_mut(entity).ok()?;
    Some((transform, velocity, health))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "value bounds are checked before casting to f32"
)]
fn f32_from_f64(value: f64) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    if value > f64::from(f32::MAX) {
        return None;
    }
    if value < f64::from(f32::MIN) {
        return None;
    }
    Some(value as f32)
}

/// Converts a `f64` axis value to `f32` and writes it to `target`, warning and
/// leaving `target` unchanged when the value is out of range. `field` names the
/// axis (e.g. `"position.x"`) for the warning's entity-specific context.
fn assign_axis(target: &mut f32, value: f64, entity: i64, field: &str) {
    match f32_from_f64(value) {
        Some(converted) => *target = converted,
        None => warn!("{field} out of range for entity {entity}"),
    }
}

/// Writes a DBSP position onto a `Transform`, warning on out-of-range axes.
fn write_position(pos: &Position, transform: &mut Transform) {
    assign_axis(
        &mut transform.translation.x,
        pos.x.into_inner(),
        pos.entity,
        "position.x",
    );
    assign_axis(
        &mut transform.translation.y,
        pos.y.into_inner(),
        pos.entity,
        "position.y",
    );
    assign_axis(
        &mut transform.translation.z,
        pos.z.into_inner(),
        pos.entity,
        "position.z",
    );
}

fn apply_positions(
    state: &DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
) {
    let positions = state.circuit.new_position_out().consolidate();
    for (pos, (), weight) in positions.iter() {
        // Only positive weights are live insertions; skip retractions and
        // zero-weight records so stale positions are never applied.
        if weight <= 0 {
            continue;
        }
        let Some((mut transform, _, _)) = resolve_write_target(state, write_query, pos.entity)
        else {
            continue;
        };
        write_position(&pos, &mut transform);
        if let Some(entry) = world_handle.entities.get_mut(&pos.entity) {
            entry.position = transform.translation;
        }
    }
}

fn apply_velocities(state: &DbspState, write_query: &mut DbspWriteQuery<'_, '_>) {
    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, (), weight) in velocities.iter() {
        // Skip retractions and zero-weight records; only apply live updates.
        if weight <= 0 {
            continue;
        }
        let Some((_, Some(mut velocity), _)) = resolve_write_target(state, write_query, vel.entity)
        else {
            continue;
        };
        assign_axis(
            &mut velocity.vx,
            vel.vx.into_inner(),
            vel.entity,
            "velocity.vx",
        );
        assign_axis(
            &mut velocity.vy,
            vel.vy.into_inner(),
            vel.entity,
            "velocity.vy",
        );
        assign_axis(
            &mut velocity.vz,
            vel.vz.into_inner(),
            vel.entity,
            "velocity.vz",
        );
    }
}

/// Resolves the entity key and mutable `Health` component for a health delta.
///
/// Returns `None` (after logging where appropriate) when the entity id cannot
/// be mapped, the entity is missing from the query, or it carries no `Health`
/// component. Extracting this guard keeps [`apply_health_deltas`] within the
/// cognitive-complexity budget without suppressing the lint.
fn resolve_health_target<'q>(
    state: &DbspState,
    write_query: &'q mut DbspWriteQuery<'_, '_>,
    delta: &HealthDelta,
) -> Option<(i64, Mut<'q, Health>)> {
    let Ok(entity_key) = i64::try_from(delta.entity) else {
        warn!("health delta for unmappable entity {}", delta.entity);
        return None;
    };
    let Some(&entity) = state.id_map.get(&entity_key) else {
        warn!("health delta for unknown entity id {}", delta.entity);
        return None;
    };
    let Ok((_, _, _, maybe_health)) = write_query.get_mut(entity) else {
        return None;
    };
    let Some(health) = maybe_health else {
        warn!("health delta received for entity without Health component");
        return None;
    };
    Some((entity_key, health))
}

fn apply_health_deltas(
    state: &mut DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
) {
    let health_deltas = state.circuit.health_delta_out().consolidate();
    for (delta, (), weight) in health_deltas.iter() {
        // Apply only positive weights; skip retractions (negative) and
        // zero-weight deltas so they never mutate Health or update
        // applied_health/world_handle.
        if weight <= 0 {
            continue;
        }
        apply_one_health_delta(state, write_query, world_handle, &delta);
    }
}

/// Applies a single positive-weight health delta: resolves the target entity's
/// `Health`, honours the dedup/retraction rules, clamps the new value, then
/// updates the component, `applied_health` bookkeeping, the world handle, and
/// the death hook. Any guard failing simply skips this delta.
fn apply_one_health_delta(
    state: &mut DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
    delta: &HealthDelta,
) {
    let key = (delta.at_tick, delta.seq);
    let Some((entity_key, mut health)) = resolve_health_target(state, write_query, delta) else {
        return;
    };
    if !should_apply_health_delta(state, delta, key) {
        return;
    }
    let Some(new_current) = clamped_health_value(delta, &health) else {
        return;
    };
    health.current = new_current;
    state.applied_health.insert(delta.entity, key);
    if let Some(entry) = world_handle.entities.get_mut(&entity_key) {
        entry.health_current = health.current;
        entry.health_max = health.max;
    }
    if delta.death {
        // Future hook: notify AI about deaths if needed.
    }
}

/// Computes the clamped `Health::current` for a delta, logging when clamping
/// occurs and returning `None` if the clamped value cannot fit in `u16`.
fn clamped_health_value(delta: &HealthDelta, health: &Health) -> Option<u16> {
    let current = i32::from(health.current);
    let max = i32::from(health.max);
    let raw = current + delta.delta;
    let new_value = raw.clamp(0, max);
    if raw != new_value {
        debug!(
            "health clamped for entity {}: raw {} -> {}",
            delta.entity, raw, new_value
        );
    }
    let Ok(value) = u16::try_from(new_value) else {
        debug_assert!(
            false,
            "clamped health value {new_value} exceeds u16 capacity"
        );
        return None;
    };
    Some(value)
}

fn should_apply_health_delta(
    state: &mut DbspState,
    delta: &HealthDelta,
    key: (Tick, Option<u32>),
) -> bool {
    if state
        .expected_health_retractions
        .remove(&(delta.entity, delta.at_tick, delta.seq))
    {
        return false;
    }
    if state.applied_health.get(&delta.entity) == Some(&key) {
        debug!(
            "duplicate health delta ignored for entity {} at tick {} seq {:?}",
            delta.entity, delta.at_tick, delta.seq
        );
        state.health_duplicate_count += 1;
        return false;
    }
    true
}

/// Applies DBSP outputs back to ECS components.
///
/// Steps the circuit, consolidates new positions and velocities, and updates
/// the corresponding entities. The [`WorldHandle`] resource is updated with the
/// latest positions for diagnostics.
///
/// Outputs are drained after application to prevent reapplying stale deltas on
/// subsequent frames.
pub fn apply_dbsp_outputs_system(
    mut commands: Commands,
    mut state: NonSendMut<DbspState>,
    mut write_query: DbspWriteQuery<'_, '_>,
    mut world_handle: ResMut<WorldHandle>,
) {
    if let Err(error) = state.step_circuit() {
        commands.trigger(DbspSyncError::new(
            DbspSyncErrorContext::Step,
            error.to_string(),
        ));
        // Clear inputs even when stepping fails so the buffered records are not
        // replayed on the next tick, then roll back the health/damage tracking
        // that the cache system advanced this frame. Clearing the inputs alone
        // would leave `health_snapshot`/`pending_damage_retractions` pointing at
        // records the circuit never accepted, corrupting next frame's
        // retractions.
        state.circuit.clear_inputs();
        state.rollback_frame_tracking();
        return;
    }

    apply_positions(&state, &mut write_query, &mut world_handle);
    apply_velocities(&state, &mut write_query);
    apply_health_deltas(&mut state, &mut write_query, &mut world_handle);
    let _ = state.circuit.health_delta_out().take_from_all();

    // Drain any remaining output so stale values are not reused.
    let _ = state.circuit.new_position_out().take_from_all();
    let _ = state.circuit.new_velocity_out().take_from_all();

    state.expected_health_retractions.clear();
    state.circuit.clear_inputs();
    // The step succeeded and its inputs are now folded into the circuit; drop
    // the pre-frame tracking backup so it cannot be rolled back later.
    state.commit_frame_tracking();
}

#[cfg(test)]
mod tests;
