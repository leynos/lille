//! Test helpers for spawn system verification.
//!
//! This module provides reusable functions for testing player and NPC spawning,
//! including entity creation, event triggering, and assertion helpers.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TiledEvent};
use lille::components::{DdlogId, Health, VelocityComp};
use lille::map::{MapSpawned, Player, PlayerSpawn, SpawnPoint};

/// Spawns a `PlayerSpawn` marker entity at the given position.
pub fn spawn_player_spawn_point(world: &mut World, position: Vec3) -> Entity {
    world
        .spawn((PlayerSpawn, Transform::from_translation(position)))
        .id()
}

/// Spawns a `SpawnPoint` marker entity at the given position.
pub fn spawn_npc_spawn_point(
    world: &mut World,
    position: Vec3,
    enemy_type: u32,
    respawn: bool,
) -> Entity {
    world
        .spawn((
            SpawnPoint {
                enemy_type,
                respawn,
            },
            Transform::from_translation(position),
        ))
        .id()
}

/// Triggers a `MapCreated` event in the world.
#[expect(deprecated, reason = "bevy_ecs_tiled 0.10 uses the legacy Event API.")]
pub fn trigger_map_created(world: &mut World) {
    world.send_event(TiledEvent::new(Entity::PLACEHOLDER, MapCreated));
}

/// Queries for the first entity with the `Player` component.
pub fn find_player(world: &mut World) -> Option<Entity> {
    let mut query = world.query_filtered::<Entity, With<Player>>();
    query.iter(world).next()
}

/// Queries for all entities with the `MapSpawned` component (excluding Player).
pub fn find_npcs(world: &mut World) -> Vec<Entity> {
    let mut query = world.query_filtered::<Entity, (With<MapSpawned>, Without<Player>)>();
    query.iter(world).collect()
}

/// Triggers the spawn system by sending a `MapCreated` event and updating the app.
pub fn execute_spawn_system(app: &mut App) {
    trigger_map_created(app.world_mut());
    app.update();
}

/// Asserts that a transform's translation matches the expected position.
pub fn assert_position_matches(transform: &Transform, expected: Vec3, entity_name: &str) {
    assert!(
        (transform.translation.x - expected.x).abs() < f32::EPSILON,
        "{entity_name} x position should match spawn point"
    );
    assert!(
        (transform.translation.y - expected.y).abs() < f32::EPSILON,
        "{entity_name} y position should match spawn point"
    );
    assert!(
        (transform.translation.z - expected.z).abs() < f32::EPSILON,
        "{entity_name} z position should match spawn point"
    );
}

/// Asserts that an entity has all required actor components.
pub fn assert_has_actor_components(world: &World, entity: Entity, entity_name: &str) {
    assert!(
        world.get::<MapSpawned>(entity).is_some(),
        "{entity_name} should have MapSpawned marker"
    );
    assert!(
        world.get::<DdlogId>(entity).is_some(),
        "{entity_name} should have DdlogId for DBSP sync"
    );
    assert!(
        world.get::<Health>(entity).is_some(),
        "{entity_name} should have Health"
    );
    assert!(
        world.get::<VelocityComp>(entity).is_some(),
        "{entity_name} should have VelocityComp"
    );
    assert!(
        world.get::<Name>(entity).is_some(),
        "{entity_name} should have Name"
    );
}

/// Asserts that a spawn point entity has been marked as consumed.
pub fn assert_spawn_consumed<T: Component>(world: &World, entity: Entity, marker_name: &str) {
    assert!(
        world.get::<T>(entity).is_some(),
        "{marker_name} should be marked as consumed"
    );
}

/// Asserts that a spawn point entity has NOT been marked as consumed.
pub fn assert_spawn_not_consumed<T: Component>(world: &World, entity: Entity, marker_name: &str) {
    assert!(
        world.get::<T>(entity).is_none(),
        "{marker_name} should NOT be marked as consumed"
    );
}
