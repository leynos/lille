#![cfg_attr(
    all(feature = "render", feature = "map"),
    doc = "Behavioural tests for `LilleMapPlugin` using rust-rspec."
)]
#![cfg_attr(
    not(all(feature = "render", feature = "map")),
    doc = "Behavioural tests require the `render` and `map` features."
)]
#![cfg(all(feature = "render", feature = "map"))]
//! Confirms the map plugin leaves the DBSP circuit authoritative when no maps
//! are loaded.

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

use std::sync::{Arc, Mutex, MutexGuard};

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use lille::{DbspPlugin, DdlogId, LilleMapPlugin, WorldHandle};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::{lock_app, SharedApp, ThreadSafeApp};

#[derive(Debug, Clone)]
struct MapPluginFixture {
    app: SharedApp,
}

impl MapPluginFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.add_plugins(DbspPlugin);
        app.add_plugins(LilleMapPlugin);

        Self {
            app: Arc::new(Mutex::new(ThreadSafeApp(app))),
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        lock_app(&self.app)
    }

    fn tick(&self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.app_guard().update();
        }));

        if let Err(payload) = result {
            bevy::log::error!(
                "tick panicked: {}",
                payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("non-string panic payload"),
            );
            std::panic::resume_unwind(payload);
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
        query.iter(world).map(|id| id.0).collect()
    }
}

#[test]
fn map_plugin_leaves_dbsp_authoritative_without_maps() {
    let fixture = MapPluginFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin runs without any loaded map assets",
        fixture,
        |scenario: &mut Scenario<MapPluginFixture>| {
            scenario.before_each(|state| state.tick());

            scenario.then("DBSP world handle stays empty without map data", |state| {
                assert_eq!(state.world_handle_entity_count(), 0);
            });

            scenario.then(
                "no DdlogId entities are inferred in the absence of maps",
                |state| {
                    assert!(state.ddlog_ids().is_empty());
                },
            );

            scenario.then(
                "subsequent ticks do not create inferred entities",
                |state| {
                    state.tick();
                    assert_eq!(state.world_handle_entity_count(), 0);
                    assert!(state.ddlog_ids().is_empty());
                },
            );
        },
    ));
}

#[test]
fn map_plugin_can_be_added_multiple_times_and_app_updates() {
    let fixture = MapPluginFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin is added twice",
        fixture,
        |scenario: &mut Scenario<MapPluginFixture>| {
            scenario.before_each(|state| {
                let mut app = state.app_guard();
                app.add_plugins(LilleMapPlugin);
            });

            scenario.then("the app can tick without panic", |state| {
                state.tick();
                state.tick();
                assert_eq!(state.world_handle_entity_count(), 0);
            });
        },
    ));
}
