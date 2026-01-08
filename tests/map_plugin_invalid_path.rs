#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` failure reporting."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Ensures invalid map configuration emits a structured error.
//!
//! This file contains a single test because it ticks the Bevy app (which can
//! initialize renderer state under `--all-features`).

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath};
use lille::LilleMapPlugin;
use rstest::rstest;

use map_test_plugins::map_test_app;
use map_test_plugins::CapturedMapErrors;

#[rstest]
fn invalid_primary_map_path_triggers_error(mut map_test_app: App) {
    map_test_app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from("/not-a-valid-asset-path.tmx"),
        should_spawn_primary_map: true,
    });

    map_test_app.add_plugins(LilleMapPlugin);
    map_test_app.finish();
    map_test_app.cleanup();
    map_test_app.update();

    let captured = map_test_app.world().resource::<CapturedMapErrors>();
    let first = captured
        .0
        .first()
        .expect("expected an invalid map asset path error to be captured");

    assert!(
        matches!(first, LilleMapError::InvalidPrimaryMapAssetPath { .. }),
        "expected InvalidPrimaryMapAssetPath error"
    );
}
