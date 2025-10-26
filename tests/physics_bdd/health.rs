//! Tests that fall damage updates Health correctly.
use bevy::prelude::*;
use crate::support::{world, TestWorld};
use lille::components::Block;
use lille::numeric::{expect_f32, floor_to_u16};
use lille::{
    Health, VelocityComp, FALL_DAMAGE_SCALE, GRAVITY_PULL, SAFE_LANDING_SPEED, TERMINAL_VELOCITY,
};
use rstest::rstest;

/// Additional margin over the safe landing speed used to model a fast fall.
const EXTRA_FALL_SPEED: f32 = 4.0;
/// Baseline health for entities in the fall-damage scenario.
const INITIAL_HEALTH: u16 = 100;

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
                        current: INITIAL_HEALTH,
                        max: INITIAL_HEALTH,
                    },
                );
                state.set_initial_health(INITIAL_HEALTH);
            });
            scenario.when("the simulation runs until the entity lands", |phase| {
                phase.before_each(|state| {
                    let fall_speed = -(expect_f32(SAFE_LANDING_SPEED) + EXTRA_FALL_SPEED);
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
                        floor_to_u16(
                            (excess * FALL_DAMAGE_SCALE)
                                .min(f64::from(u16::MAX)),
                        )
                        .expect("fall damage should fit in u16")
                    };
                    state.set_expected_damage(expected_damage);
                });
                phase.then("the expected fall damage is applied", |state| {
                    let initial_health = state.take_initial_health();
                    let expected = state.take_expected_damage();
                    let health = state.health();
                    let lost = initial_health.saturating_sub(health.current);
                    assert_eq!(lost, expected);
                });
            });
        },
    ));
}
