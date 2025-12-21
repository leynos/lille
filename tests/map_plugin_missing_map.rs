#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` failure reporting."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Ensures missing map assets emit a structured error without panicking.
//!
//! This file contains a single test because it ticks the Bevy app (which can
//! initialize renderer state under `--all-features`).

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledLayer;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath};
use lille::LilleMapPlugin;
use rstest::rstest;

use map_test_plugins::map_test_app;
use map_test_plugins::CapturedMapErrors;

const MAX_ERROR_WAIT_TICKS: usize = 200;
const TICK_SLEEP_MS: u64 = 1;

#[rstest]
fn missing_primary_map_triggers_error_and_map_does_not_load(mut map_test_app: App) {
    map_test_app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from("maps/does-not-exist.tmx"),
        should_spawn_primary_map: true,
        should_bootstrap_camera: false,
    });

    map_test_app.add_plugins(LilleMapPlugin);
    map_test_app.finish();
    map_test_app.cleanup();

    let mut load_failed = false;
    for _ in 0..MAX_ERROR_WAIT_TICKS {
        map_test_app.update();
        std::thread::sleep(std::time::Duration::from_millis(TICK_SLEEP_MS));
        if !map_test_app
            .world()
            .resource::<CapturedMapErrors>()
            .0
            .is_empty()
        {
            load_failed = true;
            break;
        }
    }

    assert!(
        load_failed,
        "expected map load failure to occur within {MAX_ERROR_WAIT_TICKS} ticks"
    );

    let world = map_test_app.world_mut();
    assert!(
        world.query::<&TiledLayer>().iter(world).next().is_none(),
        "expected no TiledLayer entities to be spawned when the primary map fails to load"
    );

    let captured = world.resource::<CapturedMapErrors>();
    let first = captured
        .0
        .first()
        .expect("expected a map load failure to be captured");
    assert!(
        matches!(first, LilleMapError::PrimaryMapLoadFailed { .. }),
        "expected PrimaryMapLoadFailed error",
    );
}
