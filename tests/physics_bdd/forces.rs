use bevy::prelude::*;
use crate::support::{physics_scenario, run_physics_scenario, world, PhysicsScenario, TestWorld};
use lille::components::{Block, ForceComp};
use lille::{VelocityComp, GRAVITY_PULL, GROUND_FRICTION};
use rstest::rstest;

fn setup_force_acceleration(world: &mut TestWorld) {
    debug_assert!(
        GROUND_FRICTION < 1.0,
        "GROUND_FRICTION must be < 1.0 for force_acceleration"
    );
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: 0,
    });
    world.spawn_block(Block {
        id: 2,
        x: 1,
        y: 0,
        z: 1,
    });
    world.spawn_entity(
        Transform::from_xyz(0.0, 0.0, 1.0),
        VelocityComp::default(),
        Some(ForceComp {
            force_x: 5.0 / (1.0 - GROUND_FRICTION),
            force_y: 0.0,
            force_z: 0.0,
            mass: Some(5.0),
        }),
    );
}

fn setup_force_mass_z(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -2,
    });
    world.spawn_entity(
        Transform::from_xyz(0.0, 0.0, 2.0),
        VelocityComp::default(),
        Some(ForceComp {
            force_x: 0.0,
            force_y: 0.0,
            force_z: 10.0,
            mass: Some(5.0),
        }),
    );
}

fn setup_invalid_mass(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -2,
    });
    world.spawn_entity(
        Transform::from_xyz(0.0, 0.0, 2.0),
        VelocityComp::default(),
        Some(ForceComp {
            force_x: 0.0,
            force_y: 0.0,
            force_z: 10.0,
            mass: Some(0.0),
        }),
    );
}

#[rstest]
#[case::force_acceleration(physics_scenario(
    setup_force_acceleration,
    (1.0, 0.0, 2.0),
    (1.0, 0.0, 0.0),
))]
#[case::force_mass_z(physics_scenario(
    setup_force_mass_z,
    (0.0, 0.0, 3.0),
    (0.0, 0.0, 1.0),
))]
#[case::invalid_mass(physics_scenario(
    setup_invalid_mass,
    (0.0, 0.0, 1.0),
    (0.0, 0.0, GRAVITY_PULL),
))]
fn force_scenarios(world: TestWorld, #[case] scenario: PhysicsScenario) {
    run_physics_scenario(world, scenario);
}
//! Tests for acceleration, Z-axis force application with mass, and invalid-mass handling.
