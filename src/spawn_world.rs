use bevy::prelude::*;

use crate::components::{DdlogId, Health, Target, UnitType};

/// Spawns a minimal demo world directly into the Bevy ECS.
pub fn spawn_world_system(mut commands: Commands) {
    let mut next_id: i64 = 1;

    // Static landmark entity
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::GRAY,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(50.0, 50.0, 0.0)),
            ..default()
        },
        DdlogId(next_id),
    ));
    next_id += 1;

    // Civilian unit with a movement target
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(125.0, 125.0, 0.0)),
            ..default()
        },
        DdlogId(next_id),
        Health(100),
        UnitType::Civvy { fraidiness: 1.0 },
        Target(Vec2::new(202.0, 200.0)),
    ));
    next_id += 1;

    // Threatening baddie
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::RED,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(150.0, 150.5, 0.0)),
            ..default()
        },
        DdlogId(next_id),
        Health(100),
        UnitType::Baddie { meanness: 10.0 },
    ));

    // Camera
    commands.spawn(Camera2dBundle::default());
}
