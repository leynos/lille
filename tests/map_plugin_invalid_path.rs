#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` failure reporting."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Ensures invalid map configuration emits a structured error.
//!
//! This file contains a single test because it ticks the Bevy app (which can
//! initialise renderer state under `--all-features`).

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath};
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
fn invalid_primary_map_path_triggers_error() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from("/not-a-valid-asset-path.tmx"),
        should_spawn_primary_map: true,
        should_bootstrap_camera: false,
    });

    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    app.update();

    let captured = app.world().resource::<CapturedMapErrors>();
    let first = captured
        .0
        .first()
        .expect("expected an invalid map asset path error to be captured");

    assert!(
        matches!(first, LilleMapError::InvalidPrimaryMapAssetPath { .. }),
        "expected InvalidPrimaryMapAssetPath error"
    );
}
