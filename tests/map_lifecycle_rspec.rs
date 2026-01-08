#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for map lifecycle using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural tests for single active map lifecycle enforcement.
//!
//! These tests verify that `LilleMapPlugin` supports safe map unloading
//! and that the DBSP state is correctly synchronised after unload.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

#[path = "support/map_fixture.rs"]
mod map_fixture;

#[path = "support/map_error_helpers.rs"]
mod map_error_helpers;

use std::sync::MutexGuard;

use bevy::prelude::*;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath, MapSpawned, UnloadPrimaryMap};
use lille::{DbspPlugin, LilleMapPlugin, WorldHandle};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

const TEST_MAP_PATH: &str = "maps/primary-isometric-custom-properties.tmx";
const MAX_LOAD_TICKS: usize = 100;

#[derive(Debug, Clone)]
struct MapLifecycleFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl MapLifecycleFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);
        map_test_plugins::install_map_error_capture(&mut app);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from(TEST_MAP_PATH),
            should_spawn_primary_map: true,
            should_bootstrap_camera: false,
        });
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

    fn tick_until_map_loaded(&self, max_ticks: usize) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            if self.map_spawned_count() > 0 {
                return true;
            }
            if !self.captured_errors().is_empty() {
                return false;
            }
        }
        false
    }

    fn trigger_unload(&self) {
        let mut app = self.app_guard();
        app.world_mut().trigger(UnloadPrimaryMap);
    }

    fn map_spawned_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<MapSpawned>>();
        query.iter(world).count()
    }

    fn captured_errors(&self) -> Vec<LilleMapError> {
        let app = self.app_guard();
        map_error_helpers::captured_errors(&app)
    }

    fn has_duplicate_map_error(&self) -> bool {
        self.captured_errors()
            .iter()
            .any(|e| matches!(e, LilleMapError::DuplicateMapAttempted { .. }))
    }

    fn world_handle_block_count(&self) -> usize {
        let app = self.app_guard();
        app.world()
            .get_resource::<WorldHandle>()
            .map_or(0, WorldHandle::block_count)
    }
}

#[test]
fn map_plugin_supports_safe_unload_and_reload() {
    let fixture = MapLifecycleFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin supports map unloading for hot reload",
        fixture,
        |scenario: &mut Scenario<MapLifecycleFixture>| {
            scenario.when("a loaded map is unloaded", |ctx| {
                ctx.then("spawned actors are despawned without errors", |state| {
                    let loaded = state.tick_until_map_loaded(MAX_LOAD_TICKS);
                    assert!(loaded, "map should load within {MAX_LOAD_TICKS} ticks");

                    state.trigger_unload();
                    state.tick();

                    assert_eq!(
                        state.map_spawned_count(),
                        0,
                        "all MapSpawned entities should be despawned"
                    );

                    assert!(
                        !state.has_duplicate_map_error(),
                        "unload should not cause duplicate map errors"
                    );

                    // Tick again to let DBSP sync run and clear the world handle.
                    state.tick();
                    assert_eq!(
                        state.world_handle_block_count(),
                        0,
                        "DBSP world handle should reflect empty state after unload"
                    );
                });
            });
        },
    ));
}
