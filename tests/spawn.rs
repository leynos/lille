#![cfg_attr(
    feature = "render",
    doc = "Integration tests covering the world-spawning Bevy system."
)]
#![cfg_attr(
    not(feature = "render"),
    doc = "Integration tests require the `render` feature."
)]
#![cfg(feature = "render")]
//! Unit tests for the world-spawning system.
//! Verifies entity counts and component assignments after system execution.
use bevy::prelude::*;
use lille::spawn_world_system;
use lille::{DdlogId, Health, Target, UnitType};

/// Tracks entity categories observed during `spawn_world_system` execution.
#[derive(Default)]
struct SpawnCounters {
    civvy: usize,
    baddie: usize,
    static_units: usize,
    cameras: usize,
}

impl SpawnCounters {
    fn record(&mut self, inputs: SpawnInputs<'_>, transform: &Transform) {
        match inputs.unit {
            Some(UnitType::Civvy { fraidiness }) => {
                self.civvy += 1;
                assert!(
                    (fraidiness - 1.0).abs() < f32::EPSILON,
                    "Unexpected Civvy fraidiness: {fraidiness}"
                );
                assert!(inputs.target.is_some(), "Civvy missing target");
                assert!(
                    transform
                        .translation
                        .abs_diff_eq(Vec3::new(125.0, 125.0, 0.0), f32::EPSILON),
                    "Unexpected Civvy position: {:?}",
                    transform.translation
                );
            }
            Some(UnitType::Baddie { meanness }) => {
                self.baddie += 1;
                assert!(
                    (meanness - 10.0).abs() < f32::EPSILON,
                    "Unexpected Baddie meanness: {meanness}"
                );
                assert!(inputs.target.is_none(), "Baddie should not have a target");
                assert!(
                    transform
                        .translation
                        .abs_diff_eq(Vec3::new(150.0, 150.5, 0.0), f32::EPSILON),
                    "Unexpected Baddie position: {:?}",
                    transform.translation
                );
            }
            None => self.record_without_unit(inputs.dd_id, transform),
        }
    }

    fn record_without_unit(&mut self, dd_id: Option<&DdlogId>, transform: &Transform) {
        if dd_id.is_none() {
            self.cameras += 1;
            return;
        }

        self.static_units += 1;
        assert!(
            transform
                .translation
                .abs_diff_eq(Vec3::new(50.0, 50.0, 0.0), f32::EPSILON),
            "Unexpected static position: {:?}",
            transform.translation
        );
    }

    fn assert_expected_totals(&self) {
        assert_eq!(self.civvy, 1);
        assert_eq!(self.baddie, 1);
        assert_eq!(self.static_units, 1);
        assert_eq!(self.cameras, 1, "Expected exactly one camera entity");
    }
}

fn assert_positive_health(entity: Entity, health: Option<&Health>) {
    if let Some(details) = health {
        assert!(
            details.current > 0,
            "Entity {entity:?} should have positive health"
        );
    }
}

#[derive(Copy, Clone)]
struct SpawnInputs<'a> {
    dd_id: Option<&'a DdlogId>,
    unit: Option<&'a UnitType>,
    target: Option<&'a Target>,
}

/// Tests that the `spawn_world_system` correctly spawns Civvy, Baddie, static, and camera entities with expected properties.
///
/// This test initialises a minimal Bevy app, runs the world-spawning system, and verifies that:
/// - Exactly one Civvy entity is spawned with `fraidiness` 1.0, a `Target` component, and position near (125.0, 125.0, 0.0).
/// - Exactly one Baddie entity is spawned with `meanness` 10.0, no `Target` component, and position near (150.0, 150.5, 0.0).
/// - Exactly one static entity is spawned at position near (50.0, 50.0, 0.0).
/// - Exactly one camera entity is present.
///
/// The test also asserts that all entities with a `Health` component have positive health.
///
#[cfg(feature = "render")]
#[test]
fn spawns_world_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, spawn_world_system);
    app.update();

    let world = app.world_mut();

    let mut counters = SpawnCounters::default();

    let mut query = world.query::<(
        Entity,
        Option<&DdlogId>,
        Option<&UnitType>,
        &Transform,
        Option<&Health>,
        Option<&Target>,
    )>();

    for (entity, dd_id, unit, transform, health, target) in query.iter(world) {
        assert_positive_health(entity, health);
        counters.record(
            SpawnInputs {
                dd_id,
                unit,
                target,
            },
            transform,
        );
    }

    counters.assert_expected_totals();
}
