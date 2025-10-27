//! Falling landings, standing on flat terrain, standing on slopes, and height propagation consistency.
use bevy::prelude::*;
use crate::support::{physics_scenario, run_physics_scenario, world, PhysicsScenario, TestWorld};
use lille::components::{Block, BlockSlope};
use lille::{VelocityComp, GRAVITY_PULL, GROUND_FRICTION};
use lille::numeric::expect_f32;
use rstest::rstest;

fn setup_falling(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -2,
    });
    world.spawn_entity_without_force(Transform::from_xyz(0.0, 0.0, 2.0), VelocityComp::default());
}

fn setup_standing_flat(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: 0,
    });
    world.spawn_entity_without_force(Transform::from_xyz(0.0, 0.0, 1.0), VelocityComp::default());
}

fn setup_standing_sloped(world: &mut TestWorld) {
    world.spawn_sloped_block(
        Block {
            id: 1,
            x: 0,
            y: 0,
            z: 0,
        },
        BlockSlope {
            block_id: 1,
            grad_x: 1.0.into(),
            grad_y: 0.0.into(),
        },
    );
    world.spawn_entity_without_force(Transform::from_xyz(0.0, 0.0, 1.5), VelocityComp::default());
}

fn setup_move_heights(world: &mut TestWorld) {
    debug_assert!(
        GROUND_FRICTION < 1.0,
        "GROUND_FRICTION must be < 1.0 for move_heights"
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
    let vx = expect_f32(1.0 / (1.0 - GROUND_FRICTION));
    world.spawn_entity_without_force(
        Transform::from_xyz(0.0, 0.0, 1.0),
        VelocityComp {
            vx,
            vy: 0.0,
            vz: 0.0,
        },
    );
}

#[rstest]
#[case::falling(physics_scenario(
    setup_falling,
    (0.0, 0.0, 1.0),
    (0.0, 0.0, GRAVITY_PULL),
))]
#[case::standing_flat(physics_scenario(
    setup_standing_flat,
    (0.0, 0.0, 1.0),
    (0.0, 0.0, 0.0),
))]
#[case::standing_sloped(physics_scenario(
    setup_standing_sloped,
    (0.0, 0.0, 1.5),
    (0.0, 0.0, 0.0),
))]
#[case::move_heights(physics_scenario(
    setup_move_heights,
    (1.0, 0.0, 2.0),
    (1.0, 0.0, 0.0),
))]
fn height_scenarios(world: TestWorld, #[case] scenario: PhysicsScenario) {
    run_physics_scenario(world, scenario);
}
