//! Systems for spawning entities into the Bevy world.
//! Provides helper functions to create sprites and initialise game objects.
use bevy::prelude::*;

use crate::components::{DdlogId, Health, Target, UnitType};

/// Creates a `SpriteBundle` with the specified colour and position.
///
/// # Parameters
///
/// - `color`: The colour to apply to the sprite.
/// - `translation`: The position of the sprite in world coordinates.
///
/// # Returns
///
/// A `SpriteBundle` with the given colour and translation, using default values for other fields.
///
/// # Examples
///
/// ```ignore
/// use bevy::prelude::*;
/// use lille::spawn_world::basic_sprite;
/// let sprite = basic_sprite(Color::RED, Vec3::new(10.0, 20.0, 0.0));
/// assert_eq!(sprite.sprite.color, Color::RED);
/// assert_eq!(sprite.transform.translation, Vec3::new(10.0, 20.0, 0.0));
/// ```
fn basic_sprite(color: Color, translation: Vec3) -> SpriteBundle {
    SpriteBundle {
        sprite: Sprite { color, ..default() },
        transform: Transform::from_translation(translation),
        ..default()
    }
}

/// Spawns a fixed set of demo entities and a camera into the Bevy ECS world.
///
/// This system creates three entities with unique IDs: a static landmark, a civilian unit with a movement target, and a hostile baddie, each with distinct properties and sprite colours. A default 2D camera is also spawned.
///
/// # Examples
///
/// ```ignore
/// use bevy::prelude::*;
/// use lille::spawn_world::spawn_world_system;
/// App::new()
///     .add_startup_system(spawn_world_system)
///     .run();
/// ```
pub fn spawn_world_system(mut commands: Commands) {
    let mut next_id: i64 = 1;

    // Static landmark entity
    commands
        .spawn(basic_sprite(Color::GRAY, Vec3::new(50.0, 50.0, 0.0)))
        .insert(DdlogId(next_id));
    next_id += 1;

    // Civilian unit with a movement target
    commands
        .spawn(basic_sprite(Color::WHITE, Vec3::new(125.0, 125.0, 0.0)))
        .insert((
            DdlogId(next_id),
            Health(100),
            UnitType::Civvy { fraidiness: 1.0 },
            Target(Vec2::new(202.0, 200.0)),
        ));
    next_id += 1;

    // Threatening baddie
    commands
        .spawn(basic_sprite(Color::RED, Vec3::new(150.0, 150.5, 0.0)))
        .insert((
            DdlogId(next_id),
            Health(100),
            UnitType::Baddie { meanness: 10.0 },
        ));
    next_id += 1;
    let _ = next_id;

    // Camera
    commands.spawn(Camera2dBundle::default());
}
