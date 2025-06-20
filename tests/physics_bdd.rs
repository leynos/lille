//! Behaviour-driven tests for physics-related systems.
//! Uses `rstest` to script scenarios covering entity transitions.
use bevy::prelude::*;
use insta::assert_ron_snapshot;
use lille::{
    apply_ddlog_deltas_system,
    components::{Block, BlockSlope, DdlogId, Health, UnitType, Velocity},
    ddlog_handle::{DdlogHandle, NewPosition, NewVelocity},
    init_ddlog_system, push_state_to_ddlog_system,
};
use rstest::rstest;

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, init_ddlog_system);
    app
}

#[rstest]
fn entity_transitions_between_standing_and_falling() {
    // GIVEN a block at z=0 and an entity standing on it
    let mut app = setup_app();
    app.add_systems(
        Update,
        (push_state_to_ddlog_system, apply_ddlog_deltas_system).chain(),
    );
    let block_entity = app
        .world
        .spawn_empty()
        .insert(Block {
            id: 1,
            x: 0,
            y: 0,
            z: 0,
        })
        .id();
    app.world.entity_mut(block_entity).insert(BlockSlope {
        block_id: 1,
        grad_x: 0.0,
        grad_y: 0.0,
    });
    app.world.spawn((
        DdlogId(1),
        Health(100),
        UnitType::Civvy { fraidiness: 0.0 },
        Transform::from_xyz(0.5, 0.5, 1.0),
        Velocity::default(),
    ));

    app.update(); // initial sync
    {
        let ddlog = app.world.resource::<DdlogHandle>();
        assert!(ddlog.deltas.is_empty());
    }

    // WHEN the block is lowered so the entity is above the floor
    app.world.entity_mut(block_entity).insert(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -1,
    });
    app.update();

    // THEN the entity should have fallen
    let ddlog = app.world.resource::<DdlogHandle>();
    assert!(ddlog.deltas[0].z < 1.0);
    let rounded: Vec<NewPosition> = ddlog
        .deltas
        .iter()
        .map(|d| NewPosition {
            entity: d.entity,
            x: (d.x * 1e4).round() / 1e4,
            y: (d.y * 1e4).round() / 1e4,
            z: (d.z * 1e4).round() / 1e4,
        })
        .collect();
    assert_ron_snapshot!("falling_delta", &rounded);
}

#[rstest]
fn force_application_updates_velocity() {
    let mut app = setup_app();
    app.add_systems(
        Update,
        (push_state_to_ddlog_system, apply_ddlog_deltas_system).chain(),
    );
    app.world.spawn((
        DdlogId(1),
        Health(100),
        UnitType::Civvy { fraidiness: 0.0 },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Velocity::default(),
    ));

    app.update();
    {
        let mut ddlog = app.world.resource_mut::<DdlogHandle>();
        ddlog.apply_force(1, Vec3::new(7.0, 0.0, 0.0));
    }
    app.update();

    let ddlog = app.world.resource::<DdlogHandle>();
    let rounded_vel: Vec<NewVelocity> = ddlog
        .velocity_deltas
        .iter()
        .map(|v| NewVelocity {
            entity: v.entity,
            vx: (v.vx * 1e4).round() / 1e4,
            vy: (v.vy * 1e4).round() / 1e4,
            vz: (v.vz * 1e4).round() / 1e4,
        })
        .collect();
    let rounded_pos: Vec<NewPosition> = ddlog
        .deltas
        .iter()
        .map(|d| NewPosition {
            entity: d.entity,
            x: (d.x * 1e4).round() / 1e4,
            y: (d.y * 1e4).round() / 1e4,
            z: (d.z * 1e4).round() / 1e4,
        })
        .collect();
    assert_ron_snapshot!("force_velocity", &rounded_vel);
    assert_ron_snapshot!("force_position", &rounded_pos);
}

#[rstest]
fn ground_friction_slows_entity() {
    let mut app = setup_app();
    app.add_systems(
        Update,
        (push_state_to_ddlog_system, apply_ddlog_deltas_system).chain(),
    );
    app.world.spawn((
        DdlogId(1),
        Health(100),
        UnitType::Civvy { fraidiness: 0.0 },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Velocity(Vec3::new(1.0, 0.0, 0.0)),
    ));

    app.update();
    app.update();

    let ddlog = app.world.resource::<DdlogHandle>();
    let rounded_vel: Vec<NewVelocity> = ddlog
        .velocity_deltas
        .iter()
        .map(|v| NewVelocity {
            entity: v.entity,
            vx: (v.vx * 1e4).round() / 1e4,
            vy: (v.vy * 1e4).round() / 1e4,
            vz: (v.vz * 1e4).round() / 1e4,
        })
        .collect();
    let rounded_pos: Vec<NewPosition> = ddlog
        .deltas
        .iter()
        .map(|d| NewPosition {
            entity: d.entity,
            x: (d.x * 1e4).round() / 1e4,
            y: (d.y * 1e4).round() / 1e4,
            z: (d.z * 1e4).round() / 1e4,
        })
        .collect();
    assert_ron_snapshot!("friction_velocity", &rounded_vel);
    assert_ron_snapshot!("friction_position", &rounded_pos);
}
