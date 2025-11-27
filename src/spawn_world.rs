//! Systems for spawning entities into the Bevy world.
//! Provides helper functions to create sprites and initialise game objects.
use bevy::log::warn;
use bevy::prelude::*;
use bevy::render::camera::OrthographicProjection;

use crate::components::{DdlogId, Health, Target, UnitType, VelocityComp};

/// Marker for the root entity that owns all demo world children.
#[derive(Component, Debug)]
pub struct WorldRoot;

/// Creates the components for a coloured sprite at the given position.
///
/// # Parameters
///
/// - `color`: The colour to apply to the sprite.
/// - `translation`: The position of the sprite in world coordinates.
///
/// # Returns
///
/// A tuple of [`Sprite`], [`Transform`], and [`Visibility`] components with the
/// specified colour and translation.
///
/// # Examples
///
/// ```ignore
/// use bevy::prelude::*;
/// use lille::spawn_world::basic_sprite;
/// let (sprite, transform, visibility) =
///     basic_sprite(Color::srgb(1.0, 0.0, 0.0), Vec3::new(10.0, 20.0, 0.0));
/// assert_eq!(sprite.color, Color::srgb(1.0, 0.0, 0.0));
/// assert_eq!(transform.translation, Vec3::new(10.0, 20.0, 0.0));
/// assert_eq!(visibility, Visibility::Visible);
/// ```
fn basic_sprite(color: Color, translation: Vec3) -> (Sprite, Transform, Visibility) {
    (
        Sprite { color, ..default() },
        Transform::from_translation(translation),
        Visibility::Visible,
    )
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
///     .add_systems(Startup, spawn_world_system)
///     .run();
/// ```
pub fn spawn_world_system(mut commands: Commands, existing_roots: Query<Entity, With<WorldRoot>>) {
    let mut next_id: i64 = 1;

    if !existing_roots.is_empty() {
        warn!("WorldRoot already present; skipping duplicate spawn_world_system");
        return;
    }

    let world_root = commands
        .spawn((
            WorldRoot,
            Name::new("demo-world-root"),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
        ))
        .id();
    // Parent all demo entities under a single root to exercise Bevy 0.16's
    // relationship lookups and keep hierarchy queries cache-friendly.

    // Static landmark entity
    commands.spawn((
        basic_sprite(Color::srgb(0.5, 0.5, 0.5), Vec3::new(50.0, 50.0, 0.0)),
        DdlogId(next_id),
        ChildOf(world_root),
    ));
    next_id += 1;

    // Civilian unit with a movement target
    commands.spawn((
        basic_sprite(Color::srgb(1.0, 1.0, 1.0), Vec3::new(125.0, 125.0, 0.0)),
        DdlogId(next_id),
        Health {
            current: 100,
            max: 100,
        },
        UnitType::Civvy { fraidiness: 1.0 },
        Target(Vec2::new(202.0, 200.0)),
        VelocityComp::default(),
        ChildOf(world_root),
    ));
    next_id += 1;

    // Threatening baddie
    commands.spawn((
        basic_sprite(Color::srgb(1.0, 0.0, 0.0), Vec3::new(150.0, 150.5, 0.0)),
        DdlogId(next_id),
        Health {
            current: 100,
            max: 100,
        },
        UnitType::Baddie { meanness: 10.0 },
        VelocityComp::default(),
        ChildOf(world_root),
    ));

    // Camera
    // Keep the camera above sprite Z so rendering matches pre-0.15 bundle defaults.
    commands.spawn((
        Camera2d,
        Projection::from(OrthographicProjection::default_2d()),
        Transform::from_xyz(0.0, 0.0, 999.9),
        Visibility::Visible,
        ChildOf(world_root),
    ));
}
