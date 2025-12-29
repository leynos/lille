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

#[path = "support/spawn_test_helpers.rs"]
mod spawn_test_helpers;

use bevy::prelude::*;
use lille::components::{DdlogId, UnitType};
use lille::map::{Player, PlayerSpawnConsumed, SpawnPointConsumed};
use lille::LilleMapPlugin;
use rstest::{fixture, rstest};
use spawn_test_helpers::{
    assert_has_actor_components, assert_position_matches, assert_spawn_consumed,
    assert_spawn_not_consumed, execute_spawn_system, find_npcs, find_player, spawn_npc_spawn_point,
    spawn_player_spawn_point,
};

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
#[case(0, "Civvy")]
#[case(3, "Baddie")]
fn spawned_npc_has_correct_unit_type(
    mut test_app: App,
    #[case] enemy_type: u32,
    #[case] expected_type: &str,
) {
    spawn_npc_spawn_point(test_app.world_mut(), Vec3::ZERO, enemy_type, false);
    execute_spawn_system(&mut test_app);

    let npcs = find_npcs(test_app.world_mut());
    let npc = *npcs.first().expect("NPC entity should exist");
    let unit_type = test_app
        .world()
        .get::<UnitType>(npc)
        .expect("NPC should have UnitType");

    let matches_expected = match expected_type {
        "Civvy" => matches!(unit_type, UnitType::Civvy { .. }),
        "Baddie" => matches!(unit_type, UnitType::Baddie { .. }),
        _ => panic!("Unknown unit type: {expected_type}"),
    };

    assert!(
        matches_expected,
        "enemy_type {enemy_type} should map to {expected_type}"
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
