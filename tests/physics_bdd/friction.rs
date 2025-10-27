//! Tests covering standing and diagonal friction, force clamping, and unsupported velocity caps.
use crate::support::{spawn_blocks, world, TestWorld};
use bevy::prelude::*;
use lille::components::ForceComp;
use lille::numeric::expect_f32;
use lille::VelocityComp;
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

// These constants bake the current physics parameters into the expected
// outcomes, avoiding per-test recomputation. The values reflect the defaults
// in `lille::constants`: `GROUND_FRICTION = 0.1`, `GRAVITY_PULL = -1.0`, and
// `TERMINAL_VELOCITY = 12.0`.
const FRICTION_APPLIED_1_0: f64 = 0.9;
const FORCE_CLAMPED_VZ: f64 = -12.0;
const FORCE_EXPECTED_POSITION_Z: f64 = -7.0;
const UNSUPPORTED_CLAMPED_VZ: f64 = -6.0;

#[rstest]
#[case::standing(FrictionConfig {
    blocks: &[(0, 0, 0)],
    transform_z: 1.0,
    initial_velocity: (1.0, 0.0, 0.0),
    force: None,
    expected_position: (FRICTION_APPLIED_1_0, 0.0, 1.0),
    expected_velocity: (FRICTION_APPLIED_1_0, 0.0, 0.0),
})]
#[case::diagonal(FrictionConfig {
    blocks: &[(0, 0, 0)],
    transform_z: 1.0,
    initial_velocity: (1.0, 1.0, 0.0),
    force: None,
    expected_position: (FRICTION_APPLIED_1_0, FRICTION_APPLIED_1_0, 1.0),
    expected_velocity: (FRICTION_APPLIED_1_0, FRICTION_APPLIED_1_0, 0.0),
})]
#[case::force_respects_terminal_velocity(FrictionConfig {
    blocks: &[(0, 0, -10)],
    transform_z: 5.0,
    initial_velocity: (0.0, 0.0, 0.0),
    force: Some((0.0, 0.0, -100.0, Some(5.0))),
    expected_position: (0.0, 0.0, FORCE_EXPECTED_POSITION_Z),
    expected_velocity: (0.0, 0.0, FORCE_CLAMPED_VZ),
})]
#[case::unsupported_velocity_capped(FrictionConfig {
    blocks: &[(0, 0, -10)],
    transform_z: 5.0,
    initial_velocity: (0.0, 0.0, -5.0),
    force: None,
    expected_position: (0.0, 0.0, -1.0),
    expected_velocity: (0.0, 0.0, UNSUPPORTED_CLAMPED_VZ),
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
