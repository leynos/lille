use bevy::prelude::*;
use lille::{spawn_world_system, DdlogId, Health, Target, UnitType};

#[test]
/// Tests that the `spawn_world_system` correctly spawns Civvy, Baddie, static, and camera entities with expected properties.
///
/// This test initialises a minimal Bevy app, runs the world-spawning system, and verifies that:
/// - Exactly one Civvy entity is spawned with `fraidiness` 1.0, a `Target` component, and position near (125.0, 125.0, 0.0).
/// - Exactly one Baddie entity is spawned with `meanness` 10.0, no `Target` component, and position near (150.0, 150.5, 0.0).
/// - Exactly one static entity is spawned at position near (50.0, 50.0, 0.0).
/// - Exactly one camera entity is present.
/// The test also asserts that all entities with a `Health` component have positive health.
///
/// # Examples
///
/// ```
/// spawns_world_entities();
/// ```
fn spawns_world_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, spawn_world_system);
    app.update();

    let world = &mut app.world;

    let mut civvy = 0;
    let mut baddie = 0;
    let mut static_count = 0;
    let mut cameras = 0;

    let mut query = world.query::<(
        Entity,
        Option<&DdlogId>,
        Option<&UnitType>,
        &Transform,
        Option<&Health>,
        Option<&Target>,
    )>();

    for (entity, dd_id, unit, transform, health, target) in query.iter(world) {
        if let Some(h) = health {
            assert!(h.0 > 0, "Entity {:?} missing health", entity);
        }

        match unit {
            Some(UnitType::Civvy { fraidiness }) => {
                civvy += 1;
                assert!((fraidiness - 1.0).abs() < f32::EPSILON);
                assert!(target.is_some(), "Civvy missing target");
                assert!(
                    transform
                        .translation
                        .abs_diff_eq(Vec3::new(125.0, 125.0, 0.0), f32::EPSILON),
                    "Unexpected Civvy position: {:?}",
                    transform.translation
                );
            }
            Some(UnitType::Baddie { meanness }) => {
                baddie += 1;
                assert!((meanness - 10.0).abs() < f32::EPSILON);
                assert!(target.is_none());
                assert!(
                    transform
                        .translation
                        .abs_diff_eq(Vec3::new(150.0, 150.5, 0.0), f32::EPSILON),
                    "Unexpected Baddie position: {:?}",
                    transform.translation
                );
            }
            None => {
                if dd_id.is_none() {
                    cameras += 1;
                    assert!(unit.is_none());
                    continue;
                }
                static_count += 1;
                assert!(
                    transform
                        .translation
                        .abs_diff_eq(Vec3::new(50.0, 50.0, 0.0), f32::EPSILON),
                    "Unexpected static position: {:?}",
                    transform.translation
                );
            }
        }
    }

    assert_eq!(civvy, 1);
    assert_eq!(baddie, 1);
    assert_eq!(static_count, 1);
    assert_eq!(cameras, 1, "Expected exactly one camera entity");
}
