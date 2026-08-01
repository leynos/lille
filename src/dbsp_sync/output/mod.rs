//! Output application systems bridging DBSP updates back into Bevy ECS.

use bevy::prelude::*;
use log::{debug, warn};

use crate::components::{DdlogId, Health, VelocityComp};
use crate::dbsp_circuit::Position;
use crate::world_handle::WorldHandle;

use super::{DbspState, DbspSyncError, DbspSyncErrorContext};

mod health;

use health::apply_health_deltas;

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

/// Returns the number of records skipped for a non-positive Z-set weight.
fn apply_positions(
    state: &DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
) -> u64 {
    let mut skipped = 0;
    let positions = state.circuit.new_position_out().consolidate();
    for (pos, (), weight) in positions.iter() {
        // Only positive weights are live insertions; skip retractions and
        // zero-weight records so stale positions are never applied.
        if weight <= 0 {
            skipped += 1;
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
    skipped
}

/// Returns the number of records skipped for a non-positive Z-set weight.
fn apply_velocities(state: &DbspState, write_query: &mut DbspWriteQuery<'_, '_>) -> u64 {
    let mut skipped = 0;
    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, (), weight) in velocities.iter() {
        // Skip retractions and zero-weight records; only apply live updates.
        if weight <= 0 {
            skipped += 1;
            continue;
        }
        // Unmapped or despawned entities are skipped silently (they are already
        // reported elsewhere), but an entity that resolves yet carries no
        // `VelocityComp` is a mismatch worth surfacing, mirroring
        // `resolve_health_target`'s missing-`Health` warning.
        let Some((_, maybe_velocity, _)) = resolve_write_target(state, write_query, vel.entity)
        else {
            continue;
        };
        let Some(mut velocity) = maybe_velocity else {
            warn!(
                "velocity update received for entity {} without VelocityComp",
                vel.entity
            );
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
    skipped
}

/// Warns for each entity whose movement decisions the circuit collapsed.
///
/// `MovementAccumulator::to_decision` is pure, so the aggregation fact travels
/// out of the circuit as a [`MovementAggregation`] record on its own output
/// handle. Reporting it is a command-layer concern, and this is where it
/// happens. Only positive-weight records are reported, matching the weight
/// gates the component writers apply.
fn report_movement_aggregations(state: &DbspState) {
    let aggregations = state.circuit.movement_aggregation_out().consolidate();
    for (aggregation, (), weight) in aggregations.iter() {
        if weight <= 0 {
            continue;
        }
        warn!(
            "aggregated {} movement decisions for entity {}, normalizing to one vector",
            aggregation.total_weight, aggregation.entity
        );
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
        state.step_failure_count += 1;
        warn!(
            "dbsp step failed; rolled back frame tracking and cleared inputs \
             (context: {:?}, failures so far: {}): {error}",
            DbspSyncErrorContext::Step,
            state.step_failure_count
        );
        return;
    }

    // Tally the records the weight gates discarded. A single bounded counter,
    // with no per-entity labels, so it stays cheap to sample every frame.
    let mut skipped = apply_positions(&state, &mut write_query, &mut world_handle);
    skipped += apply_velocities(&state, &mut write_query);
    skipped += apply_health_deltas(&mut state, &mut write_query, &mut world_handle);
    if skipped > 0 {
        state.skipped_output_count += skipped;
        debug!(
            "skipped {skipped} non-positive-weight output record(s) this frame \
             ({} total)",
            state.skipped_output_count
        );
    }
    report_movement_aggregations(&state);
    let _ = state.circuit.health_delta_out().take_from_all();

    // Drain any remaining output so stale values are not reused.
    let _ = state.circuit.new_position_out().take_from_all();
    let _ = state.circuit.new_velocity_out().take_from_all();
    let _ = state.circuit.movement_aggregation_out().take_from_all();

    state.expected_health_retractions.clear();
    state.circuit.clear_inputs();
    // The step succeeded and its inputs are now folded into the circuit; drop
    // the pre-frame tracking backup so it cannot be rolled back later.
    state.commit_frame_tracking();
}

#[cfg(test)]
mod tests;
