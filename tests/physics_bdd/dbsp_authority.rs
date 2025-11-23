//! Ensures DBSP remains the source of truth for entity state updates.
use approx::assert_relative_eq;
use bevy::prelude::*;
use lille::components::Block;
use lille::numeric::expect_f32;
use lille::{VelocityComp, GRAVITY_PULL};
use rstest::rstest;
use rspec::Scenario;

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
            add_registered_entity_case(scenario);
            add_orphan_entity_case(scenario);
            add_mirror_tampering_case(scenario);
        },
    ));
}

fn add_registered_entity_case(scenario: &mut Scenario<TestWorld>) {
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
}

fn add_orphan_entity_case(scenario: &mut Scenario<TestWorld>) {
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
}

fn add_mirror_tampering_case(scenario: &mut Scenario<TestWorld>) {
    scenario.when("the world mirror is tampered with between ticks", |phase| {
        phase.before_each(|state| {
            state.despawn_tracked_entity();
            state.spawn_entity_with_health(
                Transform::from_xyz(0.0, 0.0, 2.0),
                VelocityComp::default(),
                lille::Health {
                    current: 90,
                    max: 100,
                },
            );
            state.tick();
            {
                let mut app = state.app_guard();
                let mut handle = app.world_mut().resource_mut::<lille::WorldHandle>();
                for entry in handle.entities.values_mut() {
                    entry.position = Vec3::splat(999.0);
                    entry.health_current = 0;
                }
            }
            state.tick();
        });

        phase.then(
            "DBSP refreshes the mirror from authoritative circuit outputs",
            |state| {
                let app = state.app_guard();
                let entity = state.entity_or_panic();
                let ddlog_id = app
                    .world()
                    .get::<lille::DdlogId>(entity)
                    .expect("entity should retain DdlogId")
                    .0;
                let transform = app
                    .world()
                    .get::<Transform>(entity)
                    .expect("Transform should survive DBSP tick");
                let health = app
                    .world()
                    .get::<lille::Health>(entity)
                    .expect("Health should survive DBSP tick");
                let handle = app.world().resource::<lille::WorldHandle>();
                let entry = handle
                    .entities
                    .get(&ddlog_id)
                    .expect("WorldHandle should track entity by DdlogId");
                assert_relative_eq!(entry.position, transform.translation, epsilon = f32::EPSILON);
                assert_eq!(entry.health_current, health.current);
                assert_eq!(entry.health_max, health.max);
            },
        );
    });
}
