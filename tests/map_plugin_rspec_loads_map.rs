#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for `LilleMapPlugin` using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural test: the primary map can load without mutating DBSP state.
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
use bevy_ecs_tiled::prelude::{TiledLayer, TiledMap};
use lille::map::{LilleMapError, PRIMARY_ISOMETRIC_MAP_PATH};
use lille::{DbspPlugin, DdlogId, LilleMapPlugin, WorldHandle};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

use map_test_plugins::CapturedMapErrors;

#[derive(Debug, Clone)]
struct MapPluginFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl MapPluginFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);

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

    fn tick_until_layers_loaded(&self, max_ticks: usize) {
        for _ in 0..max_ticks {
            self.tick();
            if self.tiled_layer_count() > 0 {
                return;
            }
        }
    }

    fn world_handle_entity_count(&self) -> usize {
        let app = self.app_guard();
        app.world()
            .get_resource::<WorldHandle>()
            .map_or(0, WorldHandle::entity_count)
    }

    fn ddlog_ids(&self) -> Vec<i64> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&DdlogId>();
        query.iter(world).map(|&DdlogId(n)| n).collect()
    }

    fn tiled_map_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&TiledMap>();
        query.iter(world).count()
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
fn map_plugin_loads_primary_map_hierarchy_without_touching_dbsp() {
    let fixture = MapPluginFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin loads the primary map",
        fixture,
        |scenario: &mut Scenario<MapPluginFixture>| {
            scenario.when("the app ticks until map layers appear", |ctx| {
                ctx.before_each(|state| state.tick_until_layers_loaded(50));

                ctx.then("a single TiledMap root entity exists", |state| {
                    assert_eq!(state.tiled_map_count(), 1);
                });

                ctx.then("at least one TiledLayer entity exists", |state| {
                    assert!(state.tiled_layer_count() > 0);
                });

                ctx.then("DBSP world handle stays empty", |state| {
                    assert_eq!(state.world_handle_entity_count(), 0);
                });

                ctx.then("no DdlogId entities are inferred", |state| {
                    assert!(state.ddlog_ids().is_empty());
                });

                ctx.then("no map errors are emitted", |state| {
                    assert!(
                        state.captured_map_errors().is_empty(),
                        "expected no LilleMapError events for {PRIMARY_ISOMETRIC_MAP_PATH}"
                    );
                });
            });
        },
    ));
}
