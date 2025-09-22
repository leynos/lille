//! Output application systems bridging DBSP updates back into Bevy ECS.

use std::convert::TryFrom;

use bevy::prelude::*;
use log::{debug, error, warn};

use crate::components::{DdlogId, Health, VelocityComp};
use crate::dbsp_circuit::try_step;
use crate::world_handle::WorldHandle;

use super::DbspState;

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
                if let Some(entry) = world_handle.entities.get_mut(&pos.entity) {
                    entry.position = transform.translation;
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
