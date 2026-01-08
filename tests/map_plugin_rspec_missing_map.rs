#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for `LilleMapPlugin` using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural test: missing map assets emit a structured error.
//!
//! This file contains a single test because it ticks the Bevy app under
//! `--all-features`, which initializes a render device and uses process-global
//! renderer state.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

#[path = "support/map_fixture.rs"]
mod map_fixture;

use std::sync::MutexGuard;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledLayer;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath};
use lille::{DbspPlugin, LilleMapPlugin};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

use map_test_plugins::CapturedMapErrors;

const MAX_ERROR_WAIT_TICKS: usize = 200;

#[derive(Debug, Clone)]
struct MapPluginFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl MapPluginFixture {
    fn bootstrap_missing_map() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from("maps/does-not-exist.tmx"),
            should_spawn_primary_map: true,
        });

        map_test_plugins::install_map_error_capture(&mut app);
        app.add_plugins(LilleMapPlugin);

        Self {
            base: map_fixture::MapPluginFixtureBase::new(app),
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        self.base.app_guard()
    }

    fn tick(&self) {
        self.base.tick();
    }

    fn tick_until_map_error(&self, max_ticks: usize) {
        for _ in 0..max_ticks {
            self.tick();
            if !self.captured_map_errors().is_empty() {
                return;
            }
        }
    }

    fn tiled_layer_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&TiledLayer>();
        query.iter(world).count()
    }

    fn captured_map_errors(&self) -> Vec<LilleMapError> {
        let app = self.app_guard();
        app.world()
            .get_resource::<CapturedMapErrors>()
            .map(|errors| errors.0.clone())
            .unwrap_or_default()
    }
}

#[test]
fn map_plugin_reports_missing_primary_map_and_does_not_panic() {
    let fixture = MapPluginFixture::bootstrap_missing_map();

    run_serial(&rspec::given(
        "LilleMapPlugin is configured with a missing map asset",
        fixture,
        |scenario: &mut Scenario<MapPluginFixture>| {
            scenario.then("ticking emits a map error and spawns no layers", |state| {
                state.tick_until_map_error(MAX_ERROR_WAIT_TICKS);
                assert_eq!(state.tiled_layer_count(), 0);

                let errors = state.captured_map_errors();
                let first = errors
                    .first()
                    .expect("expected at least one captured map error");
                assert!(
                    matches!(first, LilleMapError::PrimaryMapLoadFailed { .. }),
                    "expected PrimaryMapLoadFailed error",
                );
            });

            scenario.then("subsequent ticks still do not panic", |state| {
                state.tick();
                state.tick();
            });
        },
    ));
}
