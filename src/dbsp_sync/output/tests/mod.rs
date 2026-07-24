//! Tests for the DBSP output application systems.

mod edge_cases;
mod failure_paths;

use super::*;
use crate::components::{Block, DdlogId, Health, UnitType};
use crate::dbsp_circuit::{try_step, DamageEvent, DamageSource, HealthState, Position, Velocity};
use crate::world_handle::DdlogEntity;
use crate::{DbspCircuit, DbspPlugin};
use bevy::ecs::prelude::On;
use bevy::ecs::system::RunSystemOnce;
use rstest::rstest;
use std::io;

mod dbsp_test_support {
    //! Support helpers capturing DBSP synchronisation errors in tests.
    use super::*;

    #[derive(Resource, Default, Debug)]
    pub struct CapturedErrors(pub Vec<(String, String)>);

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Observer systems must take On<T> by value."
    )]
    fn record_error(event: On<DbspSyncError>, mut errors: ResMut<CapturedErrors>) {
        let err = event.event();
        errors
            .0
            .push((format!("{:?}", err.context), err.detail.clone()));
    }

    pub fn install_error_observer(app: &mut App) {
        app.insert_resource(CapturedErrors::default());
        app.world_mut().add_observer(record_error);
    }
}

fn setup_app() -> Result<App, dbsp::Error> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(WorldHandle::default());
    app.world_mut().insert_non_send_resource(DbspState::new()?);
    Ok(app)
}

fn spawn_entity(app: &mut App) -> Entity {
    app.world_mut()
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

/// Wires the entity mapping shared by every priming path — world handle entry,
/// id/rev maps, and a block input — without pushing any position or velocity.
fn prime_entity_mapping(app: &mut App, entity: Entity) {
    {
        let mut world_handle = app.world_mut().resource_mut::<WorldHandle>();
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

    let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
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
}

/// Pushes a single position record with the given Z-set `weight`, so callers
/// can prime insertions or retractions without duplicating the mapping setup.
fn push_position_input(app: &mut App, position: Position, weight: i64) {
    let state = app.world_mut().non_send_resource_mut::<DbspState>();
    state.circuit.position_in().push(position, weight);
}

/// Pushes a single velocity record with the given Z-set `weight`.
fn push_velocity_input(app: &mut App, velocity: Velocity, weight: i64) {
    let state = app.world_mut().non_send_resource_mut::<DbspState>();
    state.circuit.velocity_in().push(velocity, weight);
}

fn prime_state(app: &mut App, entity: Entity) {
    prime_entity_mapping(app, entity);
    push_position_input(
        app,
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
        1,
    );
    let state = app.world_mut().non_send_resource_mut::<DbspState>();
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
    push_health_inputs_for(app, 1, current, amount);
}

fn push_health_inputs_for(app: &mut App, entity: u64, current: u16, amount: u16) {
    let state = app.world_mut().non_send_resource_mut::<DbspState>();
    state.circuit.health_state_in().push(
        HealthState {
            entity,
            current,
            max: 100,
        },
        1,
    );
    state.circuit.damage_in().push(
        DamageEvent {
            entity,
            amount,
            source: DamageSource::External,
            at_tick: 1,
            seq: Some(1),
        },
        1,
    );
}

fn force_step_error(_: &mut DbspCircuit) -> Result<(), dbsp::Error> {
    Err(dbsp::Error::IO(io::Error::other("forced failure")))
}

#[rstest]
fn applies_outputs_updates_components() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    push_health_inputs(&mut app, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let health = app
        .world()
        .entity(entity)
        .get::<Health>()
        .expect("Health component should remain after applying DBSP outputs");
    assert_eq!(health.current, 40);
    let velocity = app
        .world()
        .entity(entity)
        .get::<VelocityComp>()
        .expect("Velocity component should remain after applying DBSP outputs");
    assert!(velocity.vx < 1.0);
    let transform = app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform component should remain after applying DBSP outputs");
    assert!(transform.translation.x.abs() > f32::EPSILON);
    let world_handle = app.world().resource::<WorldHandle>();
    let entry = world_handle
        .entities
        .get(&1)
        .expect("World handle should include entity 1 after outputs apply");
    assert_eq!(entry.health_current, 40);
}

#[rstest]
fn duplicate_health_delta_is_ignored() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    push_health_inputs(&mut app, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<Health>()
            .expect("Health component should remain after initial DBSP output")
            .current,
        40
    );

    push_health_inputs(&mut app, 90, 50);
    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let state = app.world().non_send_resource::<DbspState>();
    assert_eq!(state.applied_health_duplicates(), 1);
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<Health>()
            .expect("Health component should remain after duplicate DBSP output")
            .current,
        40
    );
}

#[rstest]
fn negative_weight_position_is_not_applied() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_entity_mapping(&mut app, entity);
    // Push the position as a retraction (weight -1). The consolidated output
    // carries a negative weight and must be skipped rather than written to the
    // Transform.
    push_position_input(
        &mut app,
        Position {
            entity: 1,
            x: 5.0.into(),
            y: 5.0.into(),
            z: 1.0.into(),
        },
        -1,
    );

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let transform = app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform component should remain present");
    assert_eq!(
        transform.translation,
        Vec3::ZERO,
        "a negative-weight position must not mutate the Transform"
    );
}
