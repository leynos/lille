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
fn spawn_player_spawn_point(world: &mut World, x: f32, y: f32, z: f32) -> Entity {
    world
        .spawn((PlayerSpawn, Transform::from_xyz(x, y, z)))
        .id()
}

/// Spawns a `SpawnPoint` marker entity at the given position.
#[expect(
    clippy::too_many_arguments,
    reason = "Test helper needs position, enemy type, and respawn flag; consolidating would obscure test intent."
)]
fn spawn_npc_spawn_point(
    world: &mut World,
    x: f32,
    y: f32,
    z: f32,
    enemy_type: u32,
    respawn: bool,
) -> Entity {
    world
        .spawn((
            SpawnPoint {
                enemy_type,
                respawn,
            },
            Transform::from_xyz(x, y, z),
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

// --- Player spawning tests ---

#[rstest]
fn spawns_player_at_player_spawn_location(mut test_app: App) {
    spawn_player_spawn_point(test_app.world_mut(), 100.0, 200.0, 5.0);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    let player_entity =
        find_player(test_app.world_mut()).expect("expected Player entity to be spawned");
    let transform = test_app
        .world()
        .get::<Transform>(player_entity)
        .expect("player should have Transform");

    assert!(
        (transform.translation.x - 100.0).abs() < f32::EPSILON,
        "player x position should match spawn point"
    );
    assert!(
        (transform.translation.y - 200.0).abs() < f32::EPSILON,
        "player y position should match spawn point"
    );
    assert!(
        (transform.translation.z - 5.0).abs() < f32::EPSILON,
        "player z position should match spawn point"
    );
}

#[rstest]
fn spawned_player_has_required_components(mut test_app: App) {
    spawn_player_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    let player_entity = find_player(test_app.world_mut()).expect("expected Player entity");

    assert!(
        test_app.world().get::<Player>(player_entity).is_some(),
        "player should have Player marker"
    );
    assert!(
        test_app.world().get::<MapSpawned>(player_entity).is_some(),
        "player should have MapSpawned marker"
    );
    assert!(
        test_app.world().get::<DdlogId>(player_entity).is_some(),
        "player should have DdlogId for DBSP sync"
    );
    assert!(
        test_app.world().get::<Health>(player_entity).is_some(),
        "player should have Health"
    );
    assert!(
        test_app
            .world()
            .get::<VelocityComp>(player_entity)
            .is_some(),
        "player should have VelocityComp"
    );
    assert!(
        test_app.world().get::<Name>(player_entity).is_some(),
        "player should have Name"
    );
}

#[rstest]
fn player_spawn_is_marked_consumed(mut test_app: App) {
    let spawn_entity = spawn_player_spawn_point(test_app.world_mut(), 50.0, 75.0, 0.0);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    assert!(
        test_app
            .world()
            .get::<PlayerSpawnConsumed>(spawn_entity)
            .is_some(),
        "PlayerSpawn entity should be marked as consumed"
    );
}

#[rstest]
fn does_not_spawn_player_at_consumed_spawn(mut test_app: App) {
    let spawn_entity = spawn_player_spawn_point(test_app.world_mut(), 100.0, 100.0, 0.0);
    trigger_map_created(test_app.world_mut());
    test_app.update();

    // Verify first player exists.
    let first_player = find_player(test_app.world_mut()).expect("first player should be spawned");

    // Trigger another map created event.
    trigger_map_created(test_app.world_mut());
    test_app.update();

    // Player should still be the same, and spawn point should still be consumed.
    let second_player = find_player(test_app.world_mut()).expect("player should still exist");
    assert_eq!(
        first_player, second_player,
        "should not spawn a second player"
    );
    assert!(
        test_app
            .world()
            .get::<PlayerSpawnConsumed>(spawn_entity)
            .is_some(),
        "spawn point should remain consumed"
    );
}

#[rstest]
fn only_uses_first_player_spawn_when_multiple_exist(mut test_app: App) {
    // Spawn multiple PlayerSpawn points.
    spawn_player_spawn_point(test_app.world_mut(), 10.0, 20.0, 0.0);
    spawn_player_spawn_point(test_app.world_mut(), 30.0, 40.0, 0.0);
    spawn_player_spawn_point(test_app.world_mut(), 50.0, 60.0, 0.0);

    trigger_map_created(test_app.world_mut());
    test_app.update();

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
    spawn_npc_spawn_point(test_app.world_mut(), 150.0, 250.0, 10.0, 1, false);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    let npcs = find_npcs(test_app.world_mut());
    assert_eq!(npcs.len(), 1, "expected one NPC to be spawned");

    let npc = *npcs.first().expect("NPC entity should exist");
    let transform = test_app
        .world()
        .get::<Transform>(npc)
        .expect("NPC should have Transform");

    assert!(
        (transform.translation.x - 150.0).abs() < f32::EPSILON,
        "NPC x position should match spawn point"
    );
    assert!(
        (transform.translation.y - 250.0).abs() < f32::EPSILON,
        "NPC y position should match spawn point"
    );
    assert!(
        (transform.translation.z - 10.0).abs() < f32::EPSILON,
        "NPC z position should match spawn point"
    );
}

#[rstest]
fn spawned_npc_has_required_components(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0, 3, false);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    let npcs = find_npcs(test_app.world_mut());
    assert_eq!(npcs.len(), 1, "expected one NPC");
    let npc = *npcs.first().expect("NPC entity should exist");

    assert!(
        test_app.world().get::<MapSpawned>(npc).is_some(),
        "NPC should have MapSpawned marker"
    );
    assert!(
        test_app.world().get::<DdlogId>(npc).is_some(),
        "NPC should have DdlogId for DBSP sync"
    );
    assert!(
        test_app.world().get::<Health>(npc).is_some(),
        "NPC should have Health"
    );
    assert!(
        test_app.world().get::<VelocityComp>(npc).is_some(),
        "NPC should have VelocityComp"
    );
    assert!(
        test_app.world().get::<UnitType>(npc).is_some(),
        "NPC should have UnitType"
    );
    assert!(
        test_app.world().get::<Name>(npc).is_some(),
        "NPC should have Name"
    );
}

#[rstest]
fn spawned_npc_has_correct_unit_type_for_civvy(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0, 0, false);
    trigger_map_created(test_app.world_mut());

    test_app.update();

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
    spawn_npc_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0, 3, false);
    trigger_map_created(test_app.world_mut());

    test_app.update();

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
    let spawn_entity = spawn_npc_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0, 1, false);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    assert!(
        test_app
            .world()
            .get::<SpawnPointConsumed>(spawn_entity)
            .is_some(),
        "non-respawning SpawnPoint should be marked as consumed"
    );
}

#[rstest]
fn respawning_spawn_point_is_not_marked_consumed(mut test_app: App) {
    let spawn_entity = spawn_npc_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0, 1, true);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    assert!(
        test_app
            .world()
            .get::<SpawnPointConsumed>(spawn_entity)
            .is_none(),
        "respawning SpawnPoint should NOT be marked as consumed"
    );
}

#[rstest]
fn spawns_multiple_npcs_from_multiple_spawn_points(mut test_app: App) {
    spawn_npc_spawn_point(test_app.world_mut(), 10.0, 10.0, 0.0, 0, false);
    spawn_npc_spawn_point(test_app.world_mut(), 20.0, 20.0, 0.0, 1, false);
    spawn_npc_spawn_point(test_app.world_mut(), 30.0, 30.0, 0.0, 8, true);

    trigger_map_created(test_app.world_mut());
    test_app.update();

    let npcs = find_npcs(test_app.world_mut());
    assert_eq!(npcs.len(), 3, "should spawn three NPCs");
}

// --- ID uniqueness tests ---

#[rstest]
fn spawned_entities_have_unique_ddlog_ids(mut test_app: App) {
    spawn_player_spawn_point(test_app.world_mut(), 0.0, 0.0, 0.0);
    spawn_npc_spawn_point(test_app.world_mut(), 10.0, 10.0, 0.0, 1, false);
    spawn_npc_spawn_point(test_app.world_mut(), 20.0, 20.0, 0.0, 2, false);
    spawn_npc_spawn_point(test_app.world_mut(), 30.0, 30.0, 0.0, 3, true);

    trigger_map_created(test_app.world_mut());
    test_app.update();

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
    spawn_player_spawn_point(test_app.world_mut(), 50.0, 50.0, 0.0);
    spawn_npc_spawn_point(test_app.world_mut(), 100.0, 100.0, 0.0, 1, false);

    trigger_map_created(test_app.world_mut());
    test_app.update();

    let first_player = find_player(test_app.world_mut()).expect("player should exist");
    let first_npcs = find_npcs(test_app.world_mut());
    assert_eq!(first_npcs.len(), 1, "should have one NPC");

    // Second map created event.
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let second_player = find_player(test_app.world_mut()).expect("player should still exist");
    let second_npcs = find_npcs(test_app.world_mut());

    assert_eq!(first_player, second_player, "player should be unchanged");
    assert_eq!(
        first_npcs.len(),
        second_npcs.len(),
        "NPC count should be unchanged"
    );
}
