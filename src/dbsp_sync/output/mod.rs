//! Output application systems bridging DBSP updates back into Bevy ECS.

use std::convert::TryFrom;

use bevy::prelude::*;
use log::{debug, warn};

use crate::components::{DdlogId, Health, VelocityComp};
use crate::dbsp_circuit::{HealthDelta, Tick};
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

macro_rules! apply_output_records {
    ($state:expr, $query:expr, $records:expr, |$record:ident| $pattern:pat => $body:block) => {
        for ($record, (), _) in $records.iter() {
            let Some(&entity) = $state.id_map.get(&$record.entity) else {
                continue;
            };
            let Ok($pattern) = $query.get_mut(entity) else {
                continue;
            };
            $body
        }
    };
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

fn apply_positions(
    state: &DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
) {
    let positions = state.circuit.new_position_out().consolidate();
    apply_output_records!(state, write_query, positions, |pos| (_, mut transform, _, _) => {
        if let Some(x) = f32_from_f64(pos.x.into_inner()) {
            transform.translation.x = x;
        } else {
            warn!("position.x out of range for entity {}", pos.entity);
        }
        if let Some(y) = f32_from_f64(pos.y.into_inner()) {
            transform.translation.y = y;
        } else {
            warn!("position.y out of range for entity {}", pos.entity);
        }
        if let Some(z) = f32_from_f64(pos.z.into_inner()) {
            transform.translation.z = z;
        } else {
            warn!("position.z out of range for entity {}", pos.entity);
        }
        if let Some(entry) = world_handle.entities.get_mut(&pos.entity) {
            entry.position = transform.translation;
        }
    });
}

fn apply_velocities(state: &DbspState, write_query: &mut DbspWriteQuery<'_, '_>) {
    let velocities = state.circuit.new_velocity_out().consolidate();
    apply_output_records!(
        state,
        write_query,
        velocities,
        |vel| (_, _, Some(mut velocity), _) => {
            if let Some(vx) = f32_from_f64(vel.vx.into_inner()) {
                velocity.vx = vx;
            } else {
                warn!("velocity.vx out of range for entity {}", vel.entity);
            }
            if let Some(vy) = f32_from_f64(vel.vy.into_inner()) {
                velocity.vy = vy;
            } else {
                warn!("velocity.vy out of range for entity {}", vel.entity);
            }
            if let Some(vz) = f32_from_f64(vel.vz.into_inner()) {
                velocity.vz = vz;
            } else {
                warn!("velocity.vz out of range for entity {}", vel.entity);
            }
        }
    );
}

#[expect(
    clippy::cognitive_complexity,
    reason = "Health delta processing requires multiple early exits for data validation."
)]
fn apply_health_deltas(
    state: &mut DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
) {
    let health_deltas = state.circuit.health_delta_out().consolidate();
    for (delta, (), _) in health_deltas.iter() {
        let Ok(entity_key) = i64::try_from(delta.entity) else {
            warn!("health delta for unmappable entity {}", delta.entity);
            continue;
        };
        let Some(&entity) = state.id_map.get(&entity_key) else {
            warn!("health delta for unknown entity id {}", delta.entity);
            continue;
        };
        let Ok((_, _, _, maybe_health)) = write_query.get_mut(entity) else {
            continue;
        };
        let Some(mut health) = maybe_health else {
            warn!("health delta received for entity without Health component");
            continue;
        };
        let key = (delta.at_tick, delta.seq);
        if !should_apply_health_delta(state, &delta, key) {
            continue;
        }
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
        let Ok(new_u16) = u16::try_from(new_value) else {
            debug_assert!(
                false,
                "clamped health value {new_value} exceeds u16 capacity"
            );
            continue;
        };
        health.current = new_u16;
        state.applied_health.insert(delta.entity, key);
        if let Some(entry) = world_handle.entities.get_mut(&entity_key) {
            entry.health_current = health.current;
            entry.health_max = health.max;
        }
        if delta.death {
            // Future hook: notify AI about deaths if needed.
        }
    }
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
}

#[cfg(test)]
mod tests;
