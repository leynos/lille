#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests for single active map lifecycle enforcement."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Unit tests for single active map lifecycle enforcement.
//!
//! These tests validate that `LilleMapPlugin` guards against loading
//! multiple maps and provides safe cleanup for hot reload scenarios.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use lille::map::NpcIdCounter;
use lille::map::{
    LilleMapError, LilleMapSettings, MapAssetPath, MapSpawned, PrimaryMapAssetTracking,
    PrimaryTiledMap, UnloadPrimaryMap, PRIMARY_ISOMETRIC_MAP_PATH,
};
use lille::LilleMapPlugin;
use map_test_plugins::CapturedMapErrors;
use rstest::{fixture, rstest};

// -- Fixtures --

#[fixture]
fn test_app() -> App {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    map_test_plugins::install_map_error_capture(&mut app);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
        should_bootstrap_camera: false,
    });
    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    app
}

// -- Test helpers --

fn spawn_mock_primary_map(world: &mut World) -> Entity {
    world
        .spawn((Name::new("MockPrimaryMap"), PrimaryTiledMap))
        .id()
}

fn spawn_mock_map_spawned_entity(world: &mut World) -> Entity {
    world.spawn((Name::new("MockSpawned"), MapSpawned)).id()
}

fn captured_errors(app: &App) -> Vec<LilleMapError> {
    app.world()
        .get_resource::<CapturedMapErrors>()
        .map(|e| e.0.clone())
        .unwrap_or_default()
}

// -- Tests --

#[rstest]
fn emits_duplicate_map_error_when_external_map_present(mut test_app: App) {
    // Spawn a mock entity that matches PrimaryTiledMap query without setting tracking.
    // This simulates an external system spawning a map entity.
    spawn_mock_primary_map(test_app.world_mut());

    // Enable spawning and tick - should detect the external map and emit error.
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();

    let errors = captured_errors(&test_app);
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LilleMapError::DuplicateMapAttempted { .. })),
        "expected DuplicateMapAttempted error, got: {errors:?}"
    );
}

#[rstest]
fn existing_map_preserved_when_second_spawn_attempted(mut test_app: App) {
    let existing_map = spawn_mock_primary_map(test_app.world_mut());
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();

    assert!(
        test_app.world().get_entity(existing_map).is_ok(),
        "existing map entity should be preserved"
    );
}

#[rstest]
fn unload_despawns_map_spawned_entities(mut test_app: App) {
    let spawned = spawn_mock_map_spawned_entity(test_app.world_mut());
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    assert!(
        test_app.world().get_entity(spawned).is_err(),
        "MapSpawned entity should be despawned after unload"
    );
}

#[rstest]
fn unload_despawns_primary_map_entity(mut test_app: App) {
    let map_entity = spawn_mock_primary_map(test_app.world_mut());
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    assert!(
        test_app.world().get_entity(map_entity).is_err(),
        "PrimaryTiledMap entity should be despawned after unload"
    );
}

#[rstest]
fn unload_resets_tracking_state(mut test_app: App) {
    // Set up tracking state as if a map was loaded.
    {
        let mut tracking = test_app
            .world_mut()
            .resource_mut::<PrimaryMapAssetTracking>();
        tracking.asset_path = Some("test/map.tmx".to_owned());
        tracking.has_finalised = true;
    }
    // Spawn a map entity so unload triggers PrimaryMapUnloaded.
    spawn_mock_primary_map(test_app.world_mut());
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    let tracking = test_app.world().resource::<PrimaryMapAssetTracking>();
    assert!(
        tracking.asset_path.is_none(),
        "asset_path should be cleared after unload"
    );
    assert!(
        !tracking.has_finalised,
        "has_finalised should be reset after unload"
    );
}

#[rstest]
fn unload_preserves_npc_id_counter(mut test_app: App) {
    test_app.world_mut().resource_mut::<NpcIdCounter>().0 = 42;
    // Spawn a map entity so unload runs its logic.
    spawn_mock_primary_map(test_app.world_mut());
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    let counter = test_app.world().resource::<NpcIdCounter>();
    assert_eq!(counter.0, 42, "NpcIdCounter should persist across unloads");
}

#[rstest]
fn can_load_new_map_after_unload(mut test_app: App) {
    // Simulate an external map entity (no tracking) that triggers duplicate error.
    spawn_mock_primary_map(test_app.world_mut());

    // Unload.
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    // Attempt to spawn (should succeed without duplicate error since unload cleared state).
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();

    let errors = captured_errors(&test_app);
    assert!(
        !errors
            .iter()
            .any(|e| matches!(e, LilleMapError::DuplicateMapAttempted { .. })),
        "should not emit DuplicateMapAttempted after unload"
    );
}

#[rstest]
fn subsequent_ticks_do_not_emit_errors_after_map_loaded(mut test_app: App) {
    // Set up tracking as if a map was loaded normally.
    {
        let mut tracking = test_app
            .world_mut()
            .resource_mut::<PrimaryMapAssetTracking>();
        tracking.asset_path = Some("loaded/map.tmx".to_owned());
        tracking.has_finalised = true;
    }
    spawn_mock_primary_map(test_app.world_mut());

    // Enable spawning and tick multiple times - should not emit errors.
    {
        let mut settings = test_app.world_mut().resource_mut::<LilleMapSettings>();
        settings.should_spawn_primary_map = true;
    }
    test_app.update();
    test_app.update();
    test_app.update();

    let errors = captured_errors(&test_app);
    assert!(
        errors.is_empty(),
        "no errors should be emitted during normal operation, got: {errors:?}"
    );
}

#[rstest]
fn unload_without_map_is_no_op(mut test_app: App) {
    // No map spawned, trigger unload - should not panic or cause issues.
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    // No errors should be emitted for a no-op unload.
    let errors = captured_errors(&test_app);
    assert!(
        errors.is_empty(),
        "unload without a map should not emit errors"
    );
}

#[rstest]
fn multiple_unload_events_are_idempotent(mut test_app: App) {
    spawn_mock_primary_map(test_app.world_mut());
    spawn_mock_map_spawned_entity(test_app.world_mut());

    // Trigger multiple unloads.
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.world_mut().trigger(UnloadPrimaryMap);
    test_app.update();

    // All entities should be gone after a single tick processing all events.
    let mut map_query = test_app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryTiledMap>>();
    assert_eq!(
        map_query.iter(test_app.world()).count(),
        0,
        "all map entities should be despawned"
    );
}
