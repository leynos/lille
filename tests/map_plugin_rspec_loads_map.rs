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
use bevy_ecs_tiled::prelude::{TiledLayer, TiledMap};
use lille::map::{LilleMapError, PRIMARY_ISOMETRIC_MAP_PATH};
use lille::{DbspPlugin, DdlogId, LilleMapPlugin, WorldHandle};
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
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);

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
