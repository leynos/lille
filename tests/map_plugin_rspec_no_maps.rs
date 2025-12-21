#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for `LilleMapPlugin` using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural test: when map spawning is disabled, DBSP remains authoritative.
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
use lille::map::{LilleMapSettings, MapAssetPath, PRIMARY_ISOMETRIC_MAP_PATH};
use lille::{DbspPlugin, DdlogId, LilleMapPlugin, WorldHandle};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

#[derive(Debug, Clone)]
struct MapPluginFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl MapPluginFixture {
    fn bootstrap_with_settings(settings: LilleMapSettings) -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);
        app.insert_resource(settings);

        map_test_plugins::install_map_error_capture(&mut app);
        app.add_plugins(LilleMapPlugin);
        // Deliberately add twice to verify idempotence.
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
}

#[test]
fn map_plugin_disabled_spawn_leaves_dbsp_authoritative_and_is_idempotent() {
    let fixture = MapPluginFixture::bootstrap_with_settings(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
        should_bootstrap_camera: false,
    });

    run_serial(&rspec::given(
        "LilleMapPlugin runs with map spawning disabled",
        fixture,
        |scenario: &mut Scenario<MapPluginFixture>| {
            scenario.before_each(|state| state.tick());

            scenario.then("DBSP world handle stays empty", |state| {
                assert_eq!(state.world_handle_entity_count(), 0);
            });

            scenario.then("no DdlogId entities are inferred", |state| {
                assert!(state.ddlog_ids().is_empty());
            });

            scenario.then("subsequent ticks still do not infer entities", |state| {
                state.tick();
                assert_eq!(state.world_handle_entity_count(), 0);
                assert!(state.ddlog_ids().is_empty());
            });
        },
    ));
}
