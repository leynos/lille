#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering player and NPC spawning from map spawn points."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Verifies that `LilleMapPlugin` spawns player and NPC entities at
//! `PlayerSpawn` and `SpawnPoint` locations when the map is loaded.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TiledEvent};
use lille::components::{DdlogId, Health, UnitType, VelocityComp};
use lille::map::{
    MapSpawned, Player, PlayerSpawn, PlayerSpawnConsumed, SpawnPoint, SpawnPointConsumed,
};
use lille::LilleMapPlugin;
use rstest::{fixture, rstest};

/// Creates a minimal Bevy app configured for spawn testing.
///
/// The app includes map test plugins and the map plugin, but does not load any
/// map assets. This allows spawning mock entities directly for unit testing.
#[fixture]
fn test_app() -> App {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.add_plugins(LilleMapPlugin);
    app
}

/// Spawns a `PlayerSpawn` marker entity at the given position.
fn spawn_player_spawn_point(world: &mut World, position: Vec3) -> Entity {
    world
        .spawn((PlayerSpawn, Transform::from_translation(position)))
        .id()
}

/// Spawns a `SpawnPoint` marker entity at the given position.
fn spawn_npc_spawn_point(
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
fn trigger_map_created(world: &mut World) {
    world.send_event(TiledEvent::new(Entity::PLACEHOLDER, MapCreated));
}

/// Queries for the first entity with the `Player` component.
fn find_player(world: &mut World) -> Option<Entity> {
    let mut query = world.query_filtered::<Entity, With<Player>>();
    query.iter(world).next()
}

/// Queries for all entities with the `MapSpawned` component (excluding Player).
fn find_npcs(world: &mut World) -> Vec<Entity> {
    let mut query = world.query_filtered::<Entity, (With<MapSpawned>, Without<Player>)>();
    query.iter(world).collect()
}

/// Triggers the spawn system by sending a `MapCreated` event and updating the app.
fn execute_spawn_system(app: &mut App) {
    trigger_map_created(app.world_mut());
    app.update();
}

/// Asserts that a transform's translation matches the expected position.
fn assert_position_matches(transform: &Transform, expected: Vec3, entity_name: &str) {
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
fn assert_has_actor_components(world: &World, entity: Entity, entity_name: &str) {
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
fn assert_spawn_consumed<T: Component>(world: &World, entity: Entity, marker_name: &str) {
    assert!(
        world.get::<T>(entity).is_some(),
        "{marker_name} should be marked as consumed"
    );
}

/// Asserts that a spawn point entity has NOT been marked as consumed.
fn assert_spawn_not_consumed<T: Component>(world: &World, entity: Entity, marker_name: &str) {
    assert!(
        world.get::<T>(entity).is_none(),
        "{marker_name} should NOT be marked as consumed"
    );
}

// --- Player spawning tests ---

#[rstest]
fn spawns_player_at_player_spawn_location(mut test_app: App) {
    let expected_position = Vec3::new(100.0, 200.0, 5.0);
    spawn_player_spawn_point(test_app.world_mut(), expected_position);
    execute_spawn_system(&mut test_app);

    let player_entity =
        find_player(test_app.world_mut()).expect("expected Player entity to be spawned");
    let transform = test_app
        .world()
        .get::<Transform>(player_entity)
        .expect("player should have Transform");

    assert_position_matches(transform, expected_position, "player");
}

#[rstest]
fn spawned_player_has_required_components(mut test_app: App) {
    spawn_player_spawn_point(test_app.world_mut(), Vec3::ZERO);
    execute_spawn_system(&mut test_app);

    let player_entity = find_player(test_app.world_mut()).expect("expected Player entity");

    assert!(
        test_app.world().get::<Player>(player_entity).is_some(),
        "player should have Player marker"
    );
    assert_has_actor_components(test_app.world(), player_entity, "player");
}

#[rstest]
fn player_spawn_is_marked_consumed(mut test_app: App) {
    let spawn_entity = spawn_player_spawn_point(test_app.world_mut(), Vec3::new(50.0, 75.0, 0.0));
    execute_spawn_system(&mut test_app);

    assert_spawn_consumed::<PlayerSpawnConsumed>(
        test_app.world(),
        spawn_entity,
        "PlayerSpawn entity",
    );
}

#[rstest]
fn does_not_spawn_player_at_consumed_spawn(mut test_app: App) {
    let spawn_entity = spawn_player_spawn_point(test_app.world_mut(), Vec3::new(100.0, 100.0, 0.0));
    execute_spawn_system(&mut test_app);

    // Verify first player exists.
    let first_player = find_player(test_app.world_mut()).expect("first player should be spawned");

    // Trigger another map created event.
    execute_spawn_system(&mut test_app);

    // Player should still be the same, and spawn point should still be consumed.
    let second_player = find_player(test_app.world_mut()).expect("player should still exist");
    assert_eq!(
        first_player, second_player,
        "should not spawn a second player"
    );
    assert_spawn_consumed::<PlayerSpawnConsumed>(test_app.world(), spawn_entity, "spawn point");
}

#[rstest]
fn only_uses_first_player_spawn_when_multiple_exist(mut test_app: App) {
    // Spawn multiple PlayerSpawn points.
    spawn_player_spawn_point(test_app.world_mut(), Vec3::new(10.0, 20.0, 0.0));
    spawn_player_spawn_point(test_app.world_mut(), Vec3::new(30.0, 40.0, 0.0));
    spawn_player_spawn_point(test_app.world_mut(), Vec3::new(50.0, 60.0, 0.0));

    execute_spawn_system(&mut test_app);

    // Should only spawn one player.
    let mut player_query = test_app
        .world_mut()
        .query_filtered::<Entity, With<Player>>();
    let player_count = player_query.iter(test_app.world()).count();

    assert_eq!(player_count, 1, "should spawn exactly one player");
}

// --- NPC spawning tests ---

#[rstest]
fn spawns_npc_at_spawn_point_location(mut test_app: App) {
    let expected_position = Vec3::new(150.0, 250.0, 10.0);
    spawn_npc_spawn_point(test_app.world_mut(), expected_position, 1, false);
    execute_spawn_system(&mut test_app);

    let npcs = find_npcs(test_app.world_mut());
    assert_eq!(npcs.len(), 1, "expected one NPC to be spawned");

    let npc = *npcs.first().expect("NPC entity should exist");
    let transform = test_app
        .world()
        .get::<Transform>(npc)
        .expect("NPC should have Transform");

    assert_position_matches(transform, expected_position, "NPC");
}

#[rstest]
fn spawned_npc_has_required_components(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::ZERO, 3, false);
    execute_spawn_system(&mut test_app);

    let npcs = find_npcs(test_app.world_mut());
    assert_eq!(npcs.len(), 1, "expected one NPC");
    let npc = *npcs.first().expect("NPC entity should exist");

    assert_has_actor_components(test_app.world(), npc, "NPC");
    assert!(
        test_app.world().get::<UnitType>(npc).is_some(),
        "NPC should have UnitType"
    );
}

#[rstest]
fn spawned_npc_has_correct_unit_type_for_civvy(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::ZERO, 0, false);
    execute_spawn_system(&mut test_app);

    let npcs = find_npcs(test_app.world_mut());
    let npc = *npcs.first().expect("NPC entity should exist");
    let unit_type = test_app
        .world()
        .get::<UnitType>(npc)
        .expect("NPC should have UnitType");

    assert!(
        matches!(unit_type, UnitType::Civvy { .. }),
        "enemy_type 0 should map to Civvy"
    );
}

#[rstest]
fn spawned_npc_has_correct_unit_type_for_baddie(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::ZERO, 3, false);
    execute_spawn_system(&mut test_app);

    let npcs = find_npcs(test_app.world_mut());
    let npc = *npcs.first().expect("NPC entity should exist");
    let unit_type = test_app
        .world()
        .get::<UnitType>(npc)
        .expect("NPC should have UnitType");

    assert!(
        matches!(unit_type, UnitType::Baddie { .. }),
        "enemy_type 3 should map to Baddie"
    );
}

#[rstest]
fn non_respawning_spawn_point_is_marked_consumed(mut test_app: App) {
    let spawn_entity = spawn_npc_spawn_point(test_app.world_mut(), Vec3::ZERO, 1, false);
    execute_spawn_system(&mut test_app);

    assert_spawn_consumed::<SpawnPointConsumed>(
        test_app.world(),
        spawn_entity,
        "non-respawning SpawnPoint",
    );
}

#[rstest]
fn respawning_spawn_point_is_not_marked_consumed(mut test_app: App) {
    let spawn_entity = spawn_npc_spawn_point(test_app.world_mut(), Vec3::ZERO, 1, true);
    execute_spawn_system(&mut test_app);

    assert_spawn_not_consumed::<SpawnPointConsumed>(
        test_app.world(),
        spawn_entity,
        "respawning SpawnPoint",
    );
}

#[rstest]
fn spawns_multiple_npcs_from_multiple_spawn_points(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(10.0, 10.0, 0.0), 0, false);
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(20.0, 20.0, 0.0), 1, false);
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(30.0, 30.0, 0.0), 8, true);

    execute_spawn_system(&mut test_app);

    let npcs = find_npcs(test_app.world_mut());
    assert_eq!(npcs.len(), 3, "should spawn three NPCs");
}

// --- ID uniqueness tests ---

#[rstest]
fn spawned_entities_have_unique_ddlog_ids(mut test_app: App) {
    spawn_player_spawn_point(test_app.world_mut(), Vec3::ZERO);
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(10.0, 10.0, 0.0), 1, false);
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(20.0, 20.0, 0.0), 2, false);
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(30.0, 30.0, 0.0), 3, true);

    execute_spawn_system(&mut test_app);

    let player = find_player(test_app.world_mut()).expect("player should exist");
    let npcs = find_npcs(test_app.world_mut());

    let mut ids: Vec<i64> = vec![
        test_app
            .world()
            .get::<DdlogId>(player)
            .expect("player should have DdlogId")
            .0,
    ];

    for npc in npcs {
        ids.push(
            test_app
                .world()
                .get::<DdlogId>(npc)
                .expect("NPC should have DdlogId")
                .0,
        );
    }

    // Check all IDs are unique.
    let unique_count = {
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        unique.len()
    };

    assert_eq!(
        unique_count,
        ids.len(),
        "all DdlogId values should be unique"
    );
}

// --- Idempotency tests ---

#[rstest]
fn spawning_is_idempotent(mut test_app: App) {
    spawn_player_spawn_point(test_app.world_mut(), Vec3::new(50.0, 50.0, 0.0));
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::new(100.0, 100.0, 0.0), 1, false);

    execute_spawn_system(&mut test_app);

    let first_player = find_player(test_app.world_mut()).expect("player should exist");
    let first_npcs = find_npcs(test_app.world_mut());
    assert_eq!(first_npcs.len(), 1, "should have one NPC");

    // Second map created event.
    execute_spawn_system(&mut test_app);

    let second_player = find_player(test_app.world_mut()).expect("player should still exist");
    let second_npcs = find_npcs(test_app.world_mut());

    assert_eq!(first_player, second_player, "player should be unchanged");
    assert_eq!(
        first_npcs.len(),
        second_npcs.len(),
        "NPC count should be unchanged"
    );
}
