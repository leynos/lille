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
//! `--all-features`, which initialises a render device and uses process-global
//! renderer state.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

use std::sync::{Arc, Mutex, MutexGuard};

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledLayer;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath};
use lille::{DbspPlugin, LilleMapPlugin};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::{lock_app, SharedApp, ThreadSafeApp};

#[derive(Resource, Default, Debug)]
struct CapturedMapErrors(pub Vec<LilleMapError>);

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
fn record_map_error(event: On<LilleMapError>, mut captured: ResMut<CapturedMapErrors>) {
    captured.0.push(event.event().clone());
}

#[derive(Debug, Clone)]
struct MapPluginFixture {
    app: SharedApp,
}

#[derive(Resource, Debug, Default)]
struct PluginsFinalised;

impl MapPluginFixture {
    fn bootstrap_missing_map() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from("maps/does-not-exist.tmx"),
            should_spawn_primary_map: true,
            should_bootstrap_camera: false,
        });

        app.insert_resource(CapturedMapErrors::default());
        app.world_mut().add_observer(record_map_error);
        app.add_plugins(LilleMapPlugin);

        Self {
            app: Arc::new(Mutex::new(ThreadSafeApp(app))),
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        lock_app(&self.app)
    }

    fn tick(&self) {
        let mut app = self.app_guard();
        if app.world().get_resource::<PluginsFinalised>().is_none() {
            app.finish();
            app.cleanup();
            app.insert_resource(PluginsFinalised);
        }
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
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
                state.tick_until_map_error(200);
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
