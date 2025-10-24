//! Output application systems bridging DBSP updates back into Bevy ECS.

use std::convert::TryFrom;

use bevy::prelude::*;
use log::{debug, error, warn};

use crate::components::{DdlogId, Health, VelocityComp};
use crate::dbsp_circuit::try_step;
use crate::world_handle::WorldHandle;

use super::DbspState;

#[expect(
    clippy::cast_possible_truncation,
    reason = "Transforms require f32; inputs validated to remain within f32 range."
)]
fn f32_from_f64(value: f64) -> f32 {
    debug_assert!(value.is_finite(), "non-finite translation value");
    debug_assert!(
        value <= f64::from(f32::MAX),
        "translation exceeds f32 range"
    );
    debug_assert!(value >= f64::from(f32::MIN), "translation below f32 range");
    value as f32
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
        let Some(&entity) = state.id_map.get(&pos.entity) else {
            continue;
        };
        let Ok((_, mut transform, _, _)) = write_query.get_mut(entity) else {
            continue;
        };
        transform.translation.x = f32_from_f64(pos.x.into_inner());
        transform.translation.y = f32_from_f64(pos.y.into_inner());
        transform.translation.z = f32_from_f64(pos.z.into_inner());
        if let Some(entry) = world_handle.entities.get_mut(&pos.entity) {
            entry.position = transform.translation;
        }
    }

    let velocities = state.circuit.new_velocity_out().consolidate();
    for (vel, _, _) in velocities.iter() {
        let Some(&entity) = state.id_map.get(&vel.entity) else {
            continue;
        };
        let Ok((_, _, Some(mut velocity), _)) = write_query.get_mut(entity) else {
            continue;
        };
        velocity.vx = f32_from_f64(vel.vx.into_inner());
        velocity.vy = f32_from_f64(vel.vy.into_inner());
        velocity.vz = f32_from_f64(vel.vz.into_inner());
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
        let Ok((_, _, _, maybe_health)) = write_query.get_mut(entity) else {
            continue;
        };
        let Some(mut health) = maybe_health else {
            warn!("health delta received for entity without Health component");
            continue;
        };
        let key = (delta.at_tick, delta.seq);
        if state
            .expected_health_retractions
            .remove(&(delta.entity, delta.at_tick, delta.seq))
        {
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
    let _ = state.circuit.health_delta_out().take_from_all();

    // Drain any remaining output so stale values are not reused.
    let _ = state.circuit.new_position_out().take_from_all();
    let _ = state.circuit.new_velocity_out().take_from_all();

    state.expected_health_retractions.clear();
    state.circuit.clear_inputs();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Block, DdlogId, Health, UnitType};
    use crate::dbsp_circuit::{DamageEvent, DamageSource, HealthState, Position, Velocity};
    use crate::world_handle::DdlogEntity;
    use bevy_ecs::system::RunSystemOnce;
    use rstest::rstest;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(WorldHandle::default());
        app.world.insert_non_send_resource(
            DbspState::new().expect("failed to init DbspState for tests"),
        );
        app
    }

    fn spawn_entity(app: &mut App) -> Entity {
        app.world
            .spawn((
                DdlogId(1),
                Transform::default(),
                VelocityComp::default(),
                Health {
                    current: 90,
                    max: 100,
                },
            ))
            .id()
    }

    fn prime_state(app: &mut App, entity: Entity) {
        {
            let mut world_handle = app.world.resource_mut::<WorldHandle>();
            world_handle.entities.insert(
                1,
                DdlogEntity {
                    position: Vec3::ZERO,
                    unit: UnitType::Civvy { fraidiness: 0.0 },
                    health_current: 90,
                    health_max: 100,
                    target: None,
                },
            );
        }

        let mut state = app.world.non_send_resource_mut::<DbspState>();
        state.id_map.insert(1, entity);
        state.rev_map.insert(entity, 1);
        state.circuit.block_in().push(
            Block {
                id: 1,
                x: 0,
                y: 0,
                z: 0,
            },
            1,
        );
        state.circuit.position_in().push(
            Position {
                entity: 1,
                x: 0.0.into(),
                y: 0.0.into(),
                z: 1.0.into(),
            },
            1,
        );
        state.circuit.velocity_in().push(
            Velocity {
                entity: 1,
                vx: 1.0.into(),
                vy: 0.0.into(),
                vz: 0.0.into(),
            },
            1,
        );
    }

    fn push_health_inputs(app: &mut App, current: u16, amount: u16) {
        let state = app.world.non_send_resource_mut::<DbspState>();
        state.circuit.health_state_in().push(
            HealthState {
                entity: 1,
                current,
                max: 100,
            },
            1,
        );
        state.circuit.damage_in().push(
            DamageEvent {
                entity: 1,
                amount,
                source: DamageSource::External,
                at_tick: 1,
                seq: Some(1),
            },
            1,
        );
    }

    #[rstest]
    fn applies_outputs_updates_components() {
        let mut app = setup_app();
        let entity = spawn_entity(&mut app);
        prime_state(&mut app, entity);
        push_health_inputs(&mut app, 90, 50);

        app.world.run_system_once(apply_dbsp_outputs_system);

        let health = app.world.entity(entity).get::<Health>().unwrap();
        assert_eq!(health.current, 40);
        let velocity = app.world.entity(entity).get::<VelocityComp>().unwrap();
        assert!(velocity.vx < 1.0);
        let transform = app.world.entity(entity).get::<Transform>().unwrap();
        assert!(transform.translation.x.abs() > f32::EPSILON);
        let world_handle = app.world.resource::<WorldHandle>();
        let entry = world_handle.entities.get(&1).unwrap();
        assert_eq!(entry.health_current, 40);
    }

    #[rstest]
    fn duplicate_health_delta_is_ignored() {
        let mut app = setup_app();
        let entity = spawn_entity(&mut app);
        prime_state(&mut app, entity);
        push_health_inputs(&mut app, 90, 50);

        app.world.run_system_once(apply_dbsp_outputs_system);
        assert_eq!(
            app.world.entity(entity).get::<Health>().unwrap().current,
            40
        );

        push_health_inputs(&mut app, 90, 50);
        app.world.run_system_once(apply_dbsp_outputs_system);

        let state = app.world.non_send_resource::<DbspState>();
        assert_eq!(state.applied_health_duplicates(), 1);
        assert_eq!(
            app.world.entity(entity).get::<Health>().unwrap().current,
            40
        );
    }
}
