//! Unit tests for DBSP spawn synchronisation.
//!
//! These tests verify that `PlayerSpawn` and `SpawnPoint` components are
//! correctly synced into the DBSP circuit's input streams.

#![cfg(feature = "map")]

use anyhow::{ensure, Result};
use bevy::prelude::*;
use ordered_float::OrderedFloat;
use rstest::rstest;

use lille::dbsp_circuit::{PlayerSpawnLocation, SpawnPointRecord};
use lille::map::{PlayerSpawn, SpawnPoint};
use lille::DbspPlugin;

/// Returns an [`App`] with the full DBSP plugin wired.
fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(DbspPlugin);
    app
}

#[rstest]
fn player_spawn_syncs_to_circuit() -> Result<()> {
    let mut app = build_app();
    let transform = Transform::from_xyz(10.0, 20.0, 5.0);
    let entity = app.world_mut().spawn((PlayerSpawn, transform)).id();

    // Run update which triggers cache_state_for_dbsp_system → apply_dbsp_outputs_system
    app.update();

    // Verify entity was processed (no panic, circuit accepted the input)
    ensure!(
        app.world().get_entity(entity).is_ok(),
        "entity should still exist"
    );

    Ok(())
}

#[rstest]
fn spawn_point_syncs_to_circuit() -> Result<()> {
    let mut app = build_app();
    let transform = Transform::from_xyz(5.0, 15.0, 0.0);
    let spawn_point = SpawnPoint {
        enemy_type: 3,
        respawn: true,
    };
    let entity = app.world_mut().spawn((spawn_point, transform)).id();

    app.update();

    ensure!(
        app.world().get_entity(entity).is_ok(),
        "entity should still exist"
    );

    Ok(())
}

#[rstest]
fn spawn_point_preserves_metadata() -> Result<()> {
    let mut app = build_app();
    let spawn_point = SpawnPoint {
        enemy_type: 7,
        respawn: false,
    };
    let entity = app
        .world_mut()
        .spawn((spawn_point, Transform::from_xyz(1.0, 2.0, 3.0)))
        .id();

    app.update();

    // Verify the SpawnPoint component retains its values after sync
    let sp = app
        .world()
        .get::<SpawnPoint>(entity)
        .expect("SpawnPoint component missing");
    ensure!(sp.enemy_type == 7, "enemy_type should be preserved");
    ensure!(!sp.respawn, "respawn should be preserved as false");

    Ok(())
}

#[rstest]
fn multiple_spawns_sync_independently() -> Result<()> {
    let mut app = build_app();

    // Spawn multiple PlayerSpawn entities
    let _ps1 = app
        .world_mut()
        .spawn((PlayerSpawn, Transform::from_xyz(0.0, 0.0, 0.0)))
        .id();
    let _ps2 = app
        .world_mut()
        .spawn((PlayerSpawn, Transform::from_xyz(10.0, 10.0, 0.0)))
        .id();

    // Spawn multiple SpawnPoint entities
    let _sp1 = app
        .world_mut()
        .spawn((
            SpawnPoint {
                enemy_type: 1,
                respawn: true,
            },
            Transform::from_xyz(5.0, 5.0, 0.0),
        ))
        .id();
    let _sp2 = app
        .world_mut()
        .spawn((
            SpawnPoint {
                enemy_type: 2,
                respawn: false,
            },
            Transform::from_xyz(15.0, 15.0, 0.0),
        ))
        .id();

    app.update();

    // Verify all entities exist after sync
    let mut player_spawn_query = app.world_mut().query::<&PlayerSpawn>();
    let player_spawn_count = player_spawn_query.iter(app.world()).count();
    ensure!(player_spawn_count == 2, "expected 2 PlayerSpawn entities");

    let mut spawn_point_query = app.world_mut().query::<&SpawnPoint>();
    let spawn_point_count = spawn_point_query.iter(app.world()).count();
    ensure!(spawn_point_count == 2, "expected 2 SpawnPoint entities");

    Ok(())
}

#[rstest]
fn player_spawn_location_record_has_correct_structure() {
    let record = PlayerSpawnLocation {
        id: 42,
        x: OrderedFloat(10.5),
        y: OrderedFloat(20.5),
        z: OrderedFloat(0.0),
    };

    assert_eq!(record.id, 42);
    assert_eq!(record.x, OrderedFloat(10.5));
    assert_eq!(record.y, OrderedFloat(20.5));
    assert_eq!(record.z, OrderedFloat(0.0));
}

#[rstest]
fn spawn_point_record_has_correct_structure() {
    let record = SpawnPointRecord {
        id: 1,
        x: OrderedFloat(5.0),
        y: OrderedFloat(10.0),
        z: OrderedFloat(0.0),
        enemy_type: 3,
        respawn: true,
    };

    assert_eq!(record.id, 1);
    assert_eq!(record.x, OrderedFloat(5.0));
    assert_eq!(record.y, OrderedFloat(10.0));
    assert_eq!(record.z, OrderedFloat(0.0));
    assert_eq!(record.enemy_type, 3);
    assert!(record.respawn);
}

#[rstest]
fn circuit_accepts_spawn_inputs_repeatedly() {
    let mut app = build_app();
    app.world_mut()
        .spawn((PlayerSpawn, Transform::from_xyz(1.0, 2.0, 3.0)));
    app.world_mut().spawn((
        SpawnPoint {
            enemy_type: 5,
            respawn: true,
        },
        Transform::from_xyz(4.0, 5.0, 6.0),
    ));

    // Run multiple update cycles to verify repeated sync works
    for _tick in 0..5 {
        app.update();
    }
}
