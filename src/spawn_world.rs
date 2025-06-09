use bevy::prelude::*;

use crate::components::{DdlogId, Health, Target, UnitType};
use crate::world::GameWorld;

/// Inserts the initial `GameWorld` resource used to spawn entities.
///
/// This system does not spawn any entities; it only prepares the world
/// state for later systems.
pub fn init_world_system(mut commands: Commands) {
    commands.insert_resource(GameWorld::default());
}

/// Spawns the entities defined in the legacy `GameWorld` into the Bevy ECS.
pub fn spawn_world_system(mut commands: Commands, world: Res<GameWorld>) {
    let mut next_id: i64 = 1;

    // Spawn static entities
    for entity in &world.entities {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::GRAY,
                    ..default()
                },
                transform: Transform::from_translation(entity.position),
                ..default()
            },
            DdlogId(next_id),
        ));
        next_id += 1;
    }

    // Spawn actors
    for actor in &world.actors {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    ..default()
                },
                transform: Transform::from_translation(actor.entity.position),
                ..default()
            },
            DdlogId(next_id),
            Health(100),
            UnitType::Civvy {
                fraidiness: actor.fraidiness_factor,
            },
            Target(actor.target.truncate()),
        ));
        next_id += 1;
    }

    // Spawn bad guys
    for bad in &world.bad_guys {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::RED,
                    ..default()
                },
                transform: Transform::from_translation(bad.entity.position),
                ..default()
            },
            DdlogId(next_id),
            Health(100),
            UnitType::Baddie {
                meanness: bad.meanness,
            },
        ));
        next_id += 1;
    }

    // Camera
    commands.spawn(Camera2dBundle::default());
}
