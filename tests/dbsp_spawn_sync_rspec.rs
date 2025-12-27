#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for DBSP spawn synchronisation using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural tests: spawn components sync into DBSP circuit.
//!
//! These tests verify that `PlayerSpawn` and `SpawnPoint` components hydrated
//! from Tiled maps are correctly pushed into the DBSP circuit's input streams
//! during each tick.

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
use lille::dbsp_sync::DbspState;
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath, PlayerSpawn, SpawnPoint};
use lille::{DbspPlugin, LilleMapPlugin};
use map_test_plugins::CapturedMapErrors;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

const CUSTOM_PROPERTIES_MAP_PATH: &str = "maps/primary-isometric-custom-properties.tmx";
const MAX_LOAD_TICKS: usize = 50;

#[derive(Debug, Clone)]
struct SpawnSyncFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl SpawnSyncFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);

        map_test_plugins::install_map_error_capture(&mut app);
        app.insert_resource(LilleMapSettings {
            primary_map: MapAssetPath::from(CUSTOM_PROPERTIES_MAP_PATH),
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

    fn tick_until_spawns_loaded(&self, max_ticks: usize) -> bool {
        for _ in 0..max_ticks {
            self.tick();
            if self.spawns_ready() {
                return true;
            }
            if !self.captured_map_errors().is_empty() {
                return false;
            }
        }
        false
    }

    fn spawns_ready(&self) -> bool {
        self.player_spawn_count() > 0 && !self.spawn_points().is_empty()
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

    fn player_spawn_transforms(&self) -> Vec<Transform> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&PlayerSpawn, &Transform)>();
        query.iter(world).map(|(_, t)| *t).collect()
    }

    fn spawn_point_transforms(&self) -> Vec<(SpawnPoint, Transform)> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&SpawnPoint, &Transform)>();
        query.iter(world).map(|(sp, t)| (*sp, *t)).collect()
    }

    fn dbsp_state_exists(&self) -> bool {
        let app = self.app_guard();
        app.world().get_non_send_resource::<DbspState>().is_some()
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
fn spawn_components_sync_into_dbsp_circuit() {
    let fixture = SpawnSyncFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin spawns and DBSP sync integration",
        fixture,
        |scenario: &mut Scenario<SpawnSyncFixture>| {
            scenario.when("the app ticks until spawns are loaded", |ctx| {
                ctx.before_each(|state| {
                    let loaded = state.tick_until_spawns_loaded(MAX_LOAD_TICKS);
                    let map_errors = state.captured_map_errors();
                    assert!(
                        loaded,
                        "expected spawns to load within {MAX_LOAD_TICKS} ticks; \
                         map errors: {map_errors:?}"
                    );
                });

                ctx.then("DbspState resource exists", |state| {
                    assert!(
                        state.dbsp_state_exists(),
                        "DbspState should be initialised by DbspPlugin"
                    );
                });

                ctx.then("player spawn locations are present", |state| {
                    assert!(
                        state.player_spawn_count() > 0,
                        "expected at least one PlayerSpawn"
                    );
                });

                ctx.then("spawn points are present with metadata", |state| {
                    let spawns = state.spawn_points();
                    assert!(!spawns.is_empty(), "expected at least one SpawnPoint");

                    // Verify the fixture map's spawn point data
                    let spawn = spawns.first().expect("spawns should not be empty");
                    assert_eq!(spawn.enemy_type, 7, "expected enemy_type from fixture map");
                    assert!(spawn.respawn, "expected respawn=true from fixture map");
                });

                ctx.then("player spawns have transforms", |state| {
                    let transforms = state.player_spawn_transforms();
                    assert!(
                        !transforms.is_empty(),
                        "PlayerSpawn entities should have Transform"
                    );
                });

                ctx.then("spawn points have transforms", |state| {
                    let spawn_transforms = state.spawn_point_transforms();
                    assert!(
                        !spawn_transforms.is_empty(),
                        "SpawnPoint entities should have Transform"
                    );
                });

                ctx.then("additional tick completes without errors", |state| {
                    // One more tick to ensure sync continues to work
                    state.tick();
                    assert!(
                        state.captured_map_errors().is_empty(),
                        "no errors should occur during additional tick"
                    );
                });

                ctx.then("no map errors are emitted", |state| {
                    assert!(
                        state.captured_map_errors().is_empty(),
                        "no map errors expected"
                    );
                });
            });
        },
    ));
}
