//! Tests covering standing and diagonal friction, force clamping, and unsupported velocity caps.
use crate::support::{spawn_blocks, world, TestWorld};
use bevy::prelude::*;
use lille::components::ForceComp;
use lille::numeric::expect_f32;
use lille::{
    apply_ground_friction, VelocityComp, GRAVITY_PULL, GROUND_FRICTION, TERMINAL_VELOCITY,
};
use rstest::rstest;

/// Bundles the block layout, initial state, applied force, and expected outcomes for a friction BDD case.
struct FrictionConfig {
    blocks: &'static [(i32, i32, i32)],
    transform_z: f32,
    initial_velocity: (f32, f32, f32),
    force: Option<(f64, f64, f64, Option<f64>)>,
    expected_position: (f64, f64, f64),
    expected_velocity: (f64, f64, f64),
}

/// Applies a friction configuration by spawning blocks, creating the entity fixtures, and advancing the world one tick.
fn apply_config(world: &mut TestWorld, config: &FrictionConfig) {
    spawn_blocks(world, config.blocks);
    let transform = Transform::from_xyz(0.0, 0.0, config.transform_z);
    let velocity = VelocityComp {
        vx: config.initial_velocity.0,
        vy: config.initial_velocity.1,
        vz: config.initial_velocity.2,
    };
    let force = config.force.map(|(fx, fy, fz, mass)| ForceComp {
        force_x: fx,
        force_y: fy,
        force_z: fz,
        mass,
    });
    world.spawn_entity(transform, velocity, force);
    world.tick();
}

#[rstest]
#[case::standing(FrictionConfig {
    blocks: &[(0, 0, 0)],
    transform_z: 1.0,
    initial_velocity: (1.0, 0.0, 0.0),
    force: None,
    expected_position: (apply_ground_friction(1.0), 0.0, 1.0),
    expected_velocity: (apply_ground_friction(1.0), 0.0, 0.0),
})]
#[case::diagonal(FrictionConfig {
    blocks: &[(0, 0, 0)],
    transform_z: 1.0,
    initial_velocity: (1.0, 1.0, 0.0),
    force: None,
    expected_position: (
        apply_ground_friction(1.0),
        apply_ground_friction(1.0),
        1.0
    ),
    expected_velocity: (
        apply_ground_friction(1.0),
        apply_ground_friction(1.0),
        0.0
    ),
})]
#[case::force_respects_terminal_velocity({
    let clamped = (-20.0 + GRAVITY_PULL).clamp(-TERMINAL_VELOCITY, TERMINAL_VELOCITY);
    FrictionConfig {
        blocks: &[(0, 0, -10)],
        transform_z: 5.0,
        initial_velocity: (0.0, 0.0, 0.0),
        force: Some((0.0, 0.0, -100.0, Some(5.0))),
        expected_position: (0.0, 0.0, 5.0 + clamped),
        expected_velocity: (0.0, 0.0, clamped),
    }
})]
#[case::unsupported_velocity_capped({
    let clamped = (-5.0 + GRAVITY_PULL).clamp(-TERMINAL_VELOCITY, TERMINAL_VELOCITY);
    FrictionConfig {
        blocks: &[(0, 0, -10)],
        transform_z: 5.0,
        initial_velocity: (0.0, 0.0, -5.0),
        force: None,
        expected_position: (0.0, 0.0, 5.0 + clamped),
        expected_velocity: (0.0, 0.0, clamped),
    }
})]
fn friction_behaviour(mut world: TestWorld, #[case] config: FrictionConfig) {
    apply_config(&mut world, &config);

    world.assert_position(
        expect_f32(config.expected_position.0),
        expect_f32(config.expected_position.1),
        expect_f32(config.expected_position.2),
    );
    world.assert_velocity(
        expect_f32(config.expected_velocity.0),
        expect_f32(config.expected_velocity.1),
        expect_f32(config.expected_velocity.2),
    );
}
