#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` failure reporting."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Ensures missing map assets emit a structured error without panicking.
//!
//! This file contains a single test because it ticks the Bevy app (which can
//! initialise renderer state under `--all-features`).

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledLayer;
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
fn missing_primary_map_triggers_error_and_map_does_not_load() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from("maps/does-not-exist.tmx"),
        should_spawn_primary_map: true,
        should_bootstrap_camera: false,
    });

    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();

    let mut load_failed = false;
    for _ in 0..200 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
        if !app.world().resource::<CapturedMapErrors>().0.is_empty() {
            load_failed = true;
            break;
        }
    }

    assert!(
        load_failed,
        "expected map load failure to occur within 200 ticks"
    );

    let world = app.world_mut();
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
