//! Tests that fall damage updates Health correctly.
use bevy::prelude::*;
use crate::support::{world, TestWorld};
use lille::components::Block;
use lille::numeric::expect_f32;
use lille::{Health, VelocityComp, GRAVITY_PULL, SAFE_LANDING_SPEED};
use rstest::rstest;

/// Additional margin over the safe landing speed used to model a fast fall.
const EXTRA_FALL_SPEED: f32 = 4.0;
/// Baseline health for entities in the fall-damage scenario.
const INITIAL_HEALTH: u16 = 100;
// These precomputed values mirror the default physics configuration:
// `SAFE_LANDING_SPEED = 6.0`, `GRAVITY_PULL = -1.0`, `TERMINAL_VELOCITY = 12.0`,
// and `FALL_DAMAGE_SCALE = 4.0`. They yield an impact speed of 11.0 blocks/tick
// and an expected damage of 20 hit points for the configured fall scenario.
const EXPECTED_IMPACT_SPEED: f64 = 11.0;
const EXPECTED_DAMAGE: u16 = 20;

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
                    debug_assert!((EXPECTED_IMPACT_SPEED - (-(fall_speed_f64 + GRAVITY_PULL))).abs() < f64::EPSILON);
                    state.set_expected_damage(EXPECTED_DAMAGE);
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
