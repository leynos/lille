#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` map loading behaviour."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Loads the primary isometric map and asserts the Tiled hierarchy appears.
//!
//! This test ticks the Bevy app, which initializes a render device under
//! `--all-features`. Bevy's renderer uses process-global state, so this file
//! intentionally contains a single test.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledLayer, TiledMap};
use lille::map::PRIMARY_ISOMETRIC_MAP_PATH;
use lille::LilleMapPlugin;
use rstest::rstest;

use map_test_plugins::map_test_app;
use map_test_plugins::CapturedMapErrors;

const MAX_LOAD_TICKS: usize = 2_000;
const TICK_SLEEP_MS: u64 = 1;

#[rstest]
fn loads_primary_map_and_spawns_layers(mut map_test_app: App) {
    map_test_app.add_plugins(LilleMapPlugin);
    map_test_app.finish();
    map_test_app.cleanup();

    map_test_app.update();

    {
        let world = map_test_app.world_mut();
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
    for _ in 0..MAX_LOAD_TICKS {
        map_test_app.update();
        std::thread::sleep(std::time::Duration::from_millis(TICK_SLEEP_MS));

        if !map_test_app
            .world()
            .resource::<CapturedMapErrors>()
            .0
            .is_empty()
        {
            break;
        }

        let world = map_test_app.world_mut();
        if world.query::<&TiledLayer>().iter(world).next().is_some() {
            layer_found = true;
            break;
        }
    }

    let errors = map_test_app.world().resource::<CapturedMapErrors>();
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
