use bevy::prelude::*;
use lille::{spawn_world_system, GameWorld, Health, Target, UnitType};

#[test]
fn spawns_world_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.world.insert_resource(GameWorld::default());
    app.add_systems(Startup, spawn_world_system);
    app.update();

    let world = &mut app.world;

    let mut civvy = 0;
    let mut baddie = 0;

    // Query spawned units and verify their components
    let mut query = world.query::<(Entity, &UnitType, &Transform, &Health, Option<&Target>)>();
    for (entity, unit, transform, health, target) in query.iter(world) {
        // All units should have a Transform and Health
        assert!(health.0 > 0, "Entity {:?} missing health", entity);

        match unit {
            UnitType::Civvy { fraidiness } => {
                civvy += 1;
                assert!((fraidiness - 1.0).abs() < f32::EPSILON);
                assert!(target.is_some(), "Civvy missing target");
                assert_eq!(transform.translation, Vec3::new(125.0, 125.0, 0.0));
            }
            UnitType::Baddie { meanness } => {
                baddie += 1;
                assert!((meanness - 10.0).abs() < f32::EPSILON);
                assert!(target.is_none());
                assert_eq!(transform.translation, Vec3::new(150.0, 150.5, 0.0));
            }
        }
    }

    assert_eq!(civvy, 1);
    assert_eq!(baddie, 1);
}
