//! Behaviour-driven tests for physics and motion rules.
//!
//! These scenarios exercise the DBSP circuit via a headless Bevy app and use
//! `rust-rspec` to express expectations declaratively. The DBSP circuit is the
//! sole source of truth for inferred motion; Bevy merely applies its outputs.

mod support;

use bevy::prelude::*;
use lille::numeric::{expect_f32, expect_u16};
use lille::{
    apply_ground_friction,
    components::{Block, BlockSlope, ForceComp},
    Health, VelocityComp, FALL_DAMAGE_SCALE, GRAVITY_PULL, GROUND_FRICTION, SAFE_LANDING_SPEED,
    TERMINAL_VELOCITY,
};
use rstest::rstest;
use support::{
    assert_expected_damage, physics_scenario, run_physics_scenario, world, PhysicsScenario,
    SetupFn, TestWorld,
};

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

fn setup_force_acceleration(world: &mut TestWorld) {
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

fn setup_standing_friction(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: 0,
    });
    world.spawn_entity_without_force(
        Transform::from_xyz(0.0, 0.0, 1.0),
        VelocityComp {
            vx: 1.0,
            vy: 0.0,
            vz: 0.0,
        },
    );
}

fn setup_diagonal_friction(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: 0,
    });
    world.spawn_entity_without_force(
        Transform::from_xyz(0.0, 0.0, 1.0),
        VelocityComp {
            vx: 1.0,
            vy: 1.0,
            vz: 0.0,
        },
    );
}

fn setup_force_respects_terminal_velocity(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -10,
    });
    world.spawn_entity(
        Transform::from_xyz(0.0, 0.0, 5.0),
        VelocityComp::default(),
        Some(ForceComp {
            force_x: 0.0,
            force_y: 0.0,
            force_z: -100.0,
            mass: Some(5.0),
        }),
    );
}

fn setup_unsupported_velocity_capped(world: &mut TestWorld) {
    world.spawn_block(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -10,
    });
    world.spawn_entity_without_force(
        Transform::from_xyz(0.0, 0.0, 5.0),
        VelocityComp {
            vx: 0.0,
            vy: 0.0,
            vz: -5.0,
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
    (apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)), 0.0, 2.0),
    (apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)), 0.0, 0.0),
))]
#[case::force_acceleration(physics_scenario(
    setup_force_acceleration,
    (apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)), 0.0, 2.0),
    (apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)), 0.0, 0.0),
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
#[case::standing_friction(physics_scenario(
    setup_standing_friction,
    (apply_ground_friction(1.0), 0.0, 1.0),
    (apply_ground_friction(1.0), 0.0, 0.0),
))]
#[case::diagonal_friction(physics_scenario(
    setup_diagonal_friction,
    (apply_ground_friction(1.0), apply_ground_friction(1.0), 1.0),
    (apply_ground_friction(1.0), apply_ground_friction(1.0), 0.0),
))]
#[case::force_respects_terminal_velocity(physics_scenario(
    setup_force_respects_terminal_velocity,
    (
        0.0,
        0.0,
        5.0 + (-20.0 + GRAVITY_PULL).clamp(-TERMINAL_VELOCITY, TERMINAL_VELOCITY),
    ),
    (
        0.0,
        0.0,
        (-20.0 + GRAVITY_PULL).clamp(-TERMINAL_VELOCITY, TERMINAL_VELOCITY),
    ),
))]
#[case::unsupported_velocity_capped(physics_scenario(
    setup_unsupported_velocity_capped,
    (
        0.0,
        0.0,
        5.0 + (-5.0 + GRAVITY_PULL).clamp(-TERMINAL_VELOCITY, TERMINAL_VELOCITY),
    ),
    (
        0.0,
        0.0,
        (-5.0 + GRAVITY_PULL).clamp(-TERMINAL_VELOCITY, TERMINAL_VELOCITY),
    ),
))]
fn physics_scenarios(world: TestWorld, #[case] scenario: PhysicsScenario) {
    run_physics_scenario(world, scenario);
}

#[rstest]
fn falling_inflicts_health_damage(world: TestWorld) {
    rspec::run(&rspec::given(
        "an entity falling onto level ground",
        world,
        |scenario| {
            scenario.before_each(|state| {
                state.spawn_block(Block {
                    id: 99,
                    x: 0,
                    y: 0,
                    z: 0,
                });
                state.spawn_entity_with_health(
                    Transform::from_xyz(0.0, 0.0, 10.0),
                    VelocityComp::default(),
                    Health {
                        current: 100,
                        max: 100,
                    },
                );
            });
            scenario.when("the simulation runs until the entity lands", |phase| {
                phase.before_each(|state| {
                    let fall_speed = -(expect_f32(SAFE_LANDING_SPEED) + 4.0);
                    state.set_velocity_z(fall_speed);
                    state.tick();

                    state.set_velocity_z(0.0);
                    state.set_position_z(1.0);
                    state.tick();

                    let fall_speed_f64 = f64::from(fall_speed);
                    let impact_speed =
                        (-(fall_speed_f64 + GRAVITY_PULL)).clamp(0.0, TERMINAL_VELOCITY);
                    let excess = impact_speed - SAFE_LANDING_SPEED;
                    let expected_damage = if excess <= 0.0 {
                        0
                    } else {
                        expect_u16(
                            (excess * FALL_DAMAGE_SCALE)
                                .min(f64::from(u16::MAX))
                                .floor(),
                        )
                    };
                    state.set_expected_damage(expected_damage);
                });
                phase.then("the expected fall damage is applied", |state| {
                    let expected = state.take_expected_damage();
                    let health = state.health();
                    let lost = 100u16.saturating_sub(health.current);
                    assert_eq!(lost, expected);
                });
            });
        },
    ));
}
