#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for Tiled custom properties using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural test: typed custom properties hydrate into ECS components.
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

use approx::assert_relative_eq;
use bevy::prelude::*;
use lille::map::{
    Collidable, LilleMapError, LilleMapSettings, MapAssetPath, PlayerSpawn, SlopeProperties,
    SpawnPoint,
};
use lille::{DbspPlugin, DdlogId, LilleMapPlugin, WorldHandle};
use map_test_plugins::CapturedMapErrors;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

const CUSTOM_PROPERTIES_MAP_PATH: &str = "maps/primary-isometric-custom-properties.tmx";
const MAX_LOAD_TICKS: usize = 50;

/// The fixture map uses a 2×2 tile grid; every tile carries `Collidable`.
const EXPECTED_COLLIDABLE_COUNT: usize = 4;

#[derive(Debug, Clone)]
struct MapCustomPropertiesFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl MapCustomPropertiesFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);

        map_test_plugins::install_map_error_capture(&mut app);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from(CUSTOM_PROPERTIES_MAP_PATH),
            should_spawn_primary_map: true,
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

    fn tick_until_custom_properties_loaded(&self, max_ticks: usize) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            if self.custom_properties_ready() {
                return true;
            }
            if !self.captured_map_errors().is_empty() {
                return false;
            }
        }

        false
    }

    fn custom_properties_ready(&self) -> bool {
        self.collidable_count() > 0
            && self.player_spawn_count() > 0
            && !self.spawn_points().is_empty()
    }

    fn collidable_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Collidable>();
        query.iter(world).count()
    }

    fn slope_properties_sample(&self) -> Option<SlopeProperties> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&SlopeProperties>();
        query.iter(world).copied().next()
    }

    fn player_spawn_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&PlayerSpawn>();
        query.iter(world).count()
    }

    fn spawn_points(&self) -> Vec<SpawnPoint> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&SpawnPoint>();
        query.iter(world).copied().collect()
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

    fn captured_map_errors(&self) -> Vec<LilleMapError> {
        let app = self.app_guard();
        app.world()
            .get_resource::<CapturedMapErrors>()
            .map(|errors| errors.0.clone())
            .unwrap_or_default()
    }
}

#[test]
fn map_plugin_hydrates_tiled_custom_properties() {
    let fixture = MapCustomPropertiesFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin hydrates custom properties",
        fixture,
        |scenario: &mut Scenario<MapCustomPropertiesFixture>| {
            scenario.when("the app ticks until custom properties are loaded", |ctx| {
                ctx.before_each(|state| {
                    assert!(
                        state.tick_until_custom_properties_loaded(MAX_LOAD_TICKS),
                        "expected custom properties to load within {MAX_LOAD_TICKS} ticks"
                    );
                });

                ctx.then("collidable tiles are hydrated", |state| {
                    assert_eq!(
                        state.collidable_count(),
                        EXPECTED_COLLIDABLE_COUNT,
                        "expected exactly {EXPECTED_COLLIDABLE_COUNT} collidable tiles"
                    );
                });

                ctx.then("slope values are hydrated from Tiled data", |state| {
                    let slope = state
                        .slope_properties_sample()
                        .expect("expected at least one SlopeProperties component");
                    assert_relative_eq!(slope.grad_x, 0.25);
                    assert_relative_eq!(slope.grad_y, 0.5);
                });

                ctx.then("player spawn markers are hydrated", |state| {
                    assert_eq!(state.player_spawn_count(), 1);
                });

                ctx.then("spawn point data is hydrated", |state| {
                    let spawns = state.spawn_points();
                    assert_eq!(spawns.len(), 1);
                    let spawn = spawns
                        .first()
                        .copied()
                        .expect("expected a SpawnPoint entry to be available");
                    assert_eq!(spawn.enemy_type, 7);
                    assert!(spawn.respawn);
                });

                ctx.then("unknown property types are ignored", |state| {
                    // The map defines 3 objects but only 2 have registered property types.
                    // If unknown types were incorrectly registered, we'd see more than 2.
                    let hydrated_spawn_count =
                        state.player_spawn_count() + state.spawn_points().len();
                    assert_eq!(
                        hydrated_spawn_count, 2,
                        "expected only 2 spawn components from 3 objects"
                    );
                });

                ctx.then("DBSP world handle includes spawned entities", |state| {
                    // The spawn system creates 1 player + 1 NPC = 2 entities.
                    assert_eq!(state.world_handle_entity_count(), 2);
                });

                ctx.then("spawned entities have DdlogId for DBSP sync", |state| {
                    // Player and NPC are spawned with DdlogId components.
                    assert_eq!(state.ddlog_ids().len(), 2);
                });

                ctx.then("no map errors are emitted", |state| {
                    assert!(
                        state.captured_map_errors().is_empty(),
                        "expected no LilleMapError events for {CUSTOM_PROPERTIES_MAP_PATH}"
                    );
                });
            });
        },
    ));
}
