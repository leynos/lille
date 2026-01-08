#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` configuration behaviour."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Confirms the primary map spawn can be disabled.
//!
//! This file contains a single test because it ticks the Bevy app (which can
//! initialize renderer state under `--all-features`).

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMap;
use lille::map::{LilleMapSettings, MapAssetPath, PRIMARY_ISOMETRIC_MAP_PATH};
use lille::LilleMapPlugin;
use rstest::rstest;

#[rstest]
fn does_not_spawn_primary_map_when_disabled() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
    });

    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    app.update();

    let world = app.world_mut();
    assert!(
        world.query::<&TiledMap>().iter(world).next().is_none(),
        "disabling primary map spawn should leave no TiledMap entities",
    );
}
