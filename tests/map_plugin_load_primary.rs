#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` map loading behaviour."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Loads the primary isometric map and asserts the Tiled hierarchy appears.
//!
//! This test ticks the Bevy app, which initialises a render device under
//! `--all-features`. Bevy's renderer uses process-global state, so this file
//! intentionally contains a single test.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledLayer, TiledMap};
use lille::map::{LilleMapError, PRIMARY_ISOMETRIC_MAP_PATH};
use lille::LilleMapPlugin;
use rstest::rstest;

#[derive(Resource, Default, Debug)]
struct CapturedMapErrors(pub Vec<LilleMapError>);

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
fn record_map_error(event: On<LilleMapError>, mut captured: ResMut<CapturedMapErrors>) {
    captured.0.push(event.event().clone());
}

#[rstest]
fn loads_primary_map_and_spawns_layers() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();

    app.update();

    {
        let world = app.world_mut();
        let mut query = world.query::<&TiledMap>();
        let map = query
            .iter(world)
            .next()
            .expect("expected LilleMapPlugin to spawn a TiledMap root entity");

        let asset_server = world.resource::<AssetServer>();
        let path = asset_server
            .get_path(map.0.id())
            .expect("expected spawned map handle to have an associated path");
        assert_eq!(
            path.path().to_string_lossy(),
            PRIMARY_ISOMETRIC_MAP_PATH,
            "spawned TiledMap should point at the primary map asset"
        );
    }

    let mut layer_found = false;
    for _ in 0..2_000 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(1));

        if !app.world().resource::<CapturedMapErrors>().0.is_empty() {
            break;
        }

        let world = app.world_mut();
        if world.query::<&TiledLayer>().iter(world).next().is_some() {
            layer_found = true;
            break;
        }
    }

    let errors = app.world().resource::<CapturedMapErrors>();
    assert!(
        errors.0.is_empty(),
        "expected the primary map to load without emitting LilleMapError events, but observed: {:?}",
        errors.0
    );
    assert!(
        layer_found,
        "expected at least one TiledLayer to be spawned after loading the primary map"
    );
}
