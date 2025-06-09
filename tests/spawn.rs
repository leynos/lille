use bevy::prelude::*;
use lille::Entity as LilleEntity;
use lille::{spawn_world_system, Actor, BadGuy, DdlogId, GameWorld, Health, Target, UnitType};

#[test]
fn spawns_world_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut world_state = GameWorld::default();
    world_state.entities.clear();
    world_state.actors.clear();
    world_state.bad_guys.clear();

    world_state.entities.push(LilleEntity::new(50.0, 50.0, 0.0));
    world_state.actors.push(Actor::new(
        Vec3::new(125.0, 125.0, 0.0),
        Vec3::new(202.0, 200.0, 0.0),
        5.0,
        1.0,
    ));
    world_state
        .bad_guys
        .push(BadGuy::new(150.0, 150.5, 0.0, 10.0));
    app.world.insert_resource(world_state);
    app.add_systems(Startup, spawn_world_system);
    app.update();

    let world = &mut app.world;

    let mut civvy = 0;
    let mut baddie = 0;
    let mut static_count = 0;

    // Query spawned units and verify their components
    let mut query = world.query::<(
        Entity,
        Option<&DdlogId>,
        Option<&UnitType>,
        &Transform,
        Option<&Health>,
        Option<&Target>,
    )>();
    for (entity, dd_id, unit, transform, health, target) in query.iter(world) {
        // All units should have a Transform and Health
        if let Some(h) = health {
            assert!(h.0 > 0, "Entity {:?} missing health", entity);
        }

        match unit {
            Some(UnitType::Civvy { fraidiness }) => {
                civvy += 1;
                assert!((fraidiness - 1.0).abs() < f32::EPSILON);
                assert!(target.is_some(), "Civvy missing target");
                assert_eq!(transform.translation, Vec3::new(125.0, 125.0, 0.0));
            }
            Some(UnitType::Baddie { meanness }) => {
                baddie += 1;
                assert!((meanness - 10.0).abs() < f32::EPSILON);
                assert!(target.is_none());
                assert_eq!(transform.translation, Vec3::new(150.0, 150.5, 0.0));
            }
            None => {
                // Skip camera entities
                if dd_id.is_none() {
                    continue;
                }
                static_count += 1;
                assert_eq!(transform.translation, Vec3::new(50.0, 50.0, 0.0));
            }
        }
    }

    assert_eq!(civvy, 1);
    assert_eq!(baddie, 1);
    assert_eq!(static_count, 1);
}
