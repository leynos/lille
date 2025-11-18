//! Ensures DBSP remains the source of truth for entity state updates.
use bevy::prelude::*;
use lille::components::Block;
use lille::numeric::expect_f32;
use lille::{VelocityComp, GRAVITY_PULL};
use rstest::rstest;

use crate::support::{world, TestWorld};

#[rstest]
fn dbsp_controls_entity_state(world: TestWorld) {
    rspec::run(&rspec::given(
        "a gravity-aware DBSP circuit",
        world,
        |scenario| {
            scenario.before_each(|state| {
                state.spawn_block(Block {
                    id: 7,
                    x: 0,
                    y: 0,
                    z: 0,
                });
            });

            scenario.when("an entity is registered with DBSP", |phase| {
                phase.before_each(|state| {
                    state.despawn_tracked_entity();
                    state.spawn_entity_without_force(
                        Transform::from_xyz(0.0, 0.0, 2.0),
                        VelocityComp::default(),
                    );
                    state.tick();
                });
                phase.then("the circuit applies gravity", |state| {
                    let expected_z = expect_f32(2.0 + GRAVITY_PULL);
                    state.assert_position(0.0, 0.0, expected_z);
                    let expected_vz = expect_f32(GRAVITY_PULL);
                    state.assert_velocity(0.0, 0.0, expected_vz);
                });
            });

            scenario.when("an entity lacks a DdlogId registration", |phase| {
                phase.before_each(|state| {
                    state.despawn_tracked_entity();
                    state.spawn_orphan_entity(
                        Transform::from_xyz(0.0, 0.0, 2.0),
                        VelocityComp::default(),
                    );
                    state.tick();
                });
                phase.then("the circuit leaves it unchanged", |state| {
                    state.assert_position(0.0, 0.0, 2.0);
                    state.assert_velocity(0.0, 0.0, 0.0);
                });
            });
        },
    ));
}
