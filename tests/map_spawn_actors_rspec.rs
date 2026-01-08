#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for player and NPC spawning using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural tests: player and NPC entities are spawned at map load.
//!
//! These tests tick the Bevy app under `--all-features`, which initializes a
//! render device and uses process-global renderer state. The rspec runner
//! serializes execution to avoid conflicts.

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
use lille::components::{DdlogId, Health, UnitType, VelocityComp};
use lille::map::{
    LilleMapError, LilleMapSettings, MapAssetPath, MapSpawned, Player, PlayerSpawn,
    PlayerSpawnConsumed, SpawnPoint, SpawnPointConsumed,
};
use lille::{DbspPlugin, LilleMapPlugin};
use map_test_plugins::CapturedMapErrors;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

const CUSTOM_PROPERTIES_MAP_PATH: &str = "maps/primary-isometric-custom-properties.tmx";
const MAX_LOAD_TICKS: usize = 100;

/// The fixture map has one `PlayerSpawn` and one `SpawnPoint` (with respawn=true).
const EXPECTED_PLAYER_SPAWN_COUNT: usize = 1;
const EXPECTED_SPAWN_POINT_COUNT: usize = 1;

/// The `SpawnPoint` in the fixture map has `enemy_type=7`, which maps to Elite (Baddie).
const EXPECTED_ENEMY_TYPE: u32 = 7;

#[derive(Debug, Clone)]
struct SpawnActorsFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl SpawnActorsFixture {
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

    fn tick_until_spawns_complete(&self, max_ticks: usize) -> bool {
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
        self.player_count() == EXPECTED_PLAYER_SPAWN_COUNT
            && self.npc_count() == EXPECTED_SPAWN_POINT_COUNT
    }

    fn captured_map_errors(&self) -> Vec<LilleMapError> {
        let app = self.app_guard();
        app.world()
            .get_resource::<CapturedMapErrors>()
            .map(|errors| errors.0.clone())
            .unwrap_or_default()
    }

    // --- Player queries ---

    fn player_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, With<Player>>();
        query.iter(world).count()
    }

    fn player_has_map_spawned(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, (With<Player>, With<MapSpawned>)>();
        query.iter(world).next().is_some()
    }

    fn player_has_ddlog_id(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, (With<Player>, With<DdlogId>)>();
        query.iter(world).next().is_some()
    }

    fn player_has_health(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, (With<Player>, With<Health>)>();
        query.iter(world).next().is_some()
    }

    fn player_has_velocity(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, (With<Player>, With<VelocityComp>)>();
        query.iter(world).next().is_some()
    }

    fn player_spawn_consumed_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<Entity, (With<PlayerSpawn>, With<PlayerSpawnConsumed>)>();
        query.iter(world).count()
    }

    // --- NPC queries ---

    fn npc_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<Entity, (With<MapSpawned>, Without<Player>)>();
        query.iter(world).count()
    }

    fn npc_has_unit_type(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<Entity, (With<MapSpawned>, With<UnitType>, Without<Player>)>();
        query.iter(world).next().is_some()
    }

    fn npc_is_baddie(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query_filtered::<&UnitType, (With<MapSpawned>, Without<Player>)>();
        query
            .iter(world)
            .any(|ut| matches!(ut, UnitType::Baddie { .. }))
    }

    fn npc_has_ddlog_id(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<Entity, (With<MapSpawned>, With<DdlogId>, Without<Player>)>();
        query.iter(world).next().is_some()
    }

    #[expect(
        dead_code,
        reason = "Reserved for future tests with non-respawning spawn points."
    )]
    fn spawn_point_consumed_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<Entity, (With<SpawnPoint>, With<SpawnPointConsumed>)>();
        query.iter(world).count()
    }

    fn respawning_spawn_point_not_consumed(&self) -> bool {
        // The fixture map's SpawnPoint has respawn=true, so it should NOT be consumed.
        let mut app = self.app_guard();
        let world = app.world_mut();

        let mut sp_query = world.query::<(&SpawnPoint, Option<&SpawnPointConsumed>)>();
        for (sp, consumed) in sp_query.iter(world) {
            if sp.respawn && consumed.is_some() {
                return false;
            }
        }
        true
    }

    // --- ID uniqueness ---

    fn all_ids_unique(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&DdlogId>();
        let ids: Vec<i64> = query.iter(world).map(|id| id.0).collect();

        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        unique.len() == ids.len()
    }

    // --- Spawn point validation ---

    fn spawn_point_enemy_type(&self) -> Option<u32> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&SpawnPoint>();
        query.iter(world).next().map(|sp| sp.enemy_type)
    }
}

#[test]
fn map_plugin_spawns_player_at_player_spawn() {
    let fixture = SpawnActorsFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin spawns player at PlayerSpawn locations",
        fixture,
        |scenario: &mut Scenario<SpawnActorsFixture>| {
            scenario.when("the app ticks until spawns are complete", |ctx| {
                ctx.before_each(|state| {
                    let spawned = state.tick_until_spawns_complete(MAX_LOAD_TICKS);
                    let map_errors = state.captured_map_errors();
                    assert!(
                        spawned,
                        "expected spawns to complete within {MAX_LOAD_TICKS} ticks; \
                         map errors: {map_errors:?}"
                    );
                });

                ctx.then("exactly one Player entity exists", |state| {
                    assert_eq!(
                        state.player_count(),
                        EXPECTED_PLAYER_SPAWN_COUNT,
                        "expected exactly {EXPECTED_PLAYER_SPAWN_COUNT} Player entity"
                    );
                });

                ctx.then("player has MapSpawned marker", |state| {
                    assert!(
                        state.player_has_map_spawned(),
                        "Player should have MapSpawned component"
                    );
                });

                ctx.then("player has DdlogId for DBSP sync", |state| {
                    assert!(
                        state.player_has_ddlog_id(),
                        "Player should have DdlogId component"
                    );
                });

                ctx.then("player has Health component", |state| {
                    assert!(
                        state.player_has_health(),
                        "Player should have Health component"
                    );
                });

                ctx.then("player has VelocityComp component", |state| {
                    assert!(
                        state.player_has_velocity(),
                        "Player should have VelocityComp component"
                    );
                });

                ctx.then("PlayerSpawn point is marked consumed", |state| {
                    assert_eq!(
                        state.player_spawn_consumed_count(),
                        EXPECTED_PLAYER_SPAWN_COUNT,
                        "PlayerSpawn should be marked as consumed"
                    );
                });

                ctx.then("no map errors are emitted", |state| {
                    assert!(state.captured_map_errors().is_empty());
                });
            });
        },
    ));
}

#[test]
fn map_plugin_spawns_npcs_at_spawn_points() {
    let fixture = SpawnActorsFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin spawns NPCs at SpawnPoint locations",
        fixture,
        |scenario: &mut Scenario<SpawnActorsFixture>| {
            scenario.when("the app ticks until spawns are complete", |ctx| {
                ctx.before_each(|state| {
                    let spawned = state.tick_until_spawns_complete(MAX_LOAD_TICKS);
                    let map_errors = state.captured_map_errors();
                    assert!(
                        spawned,
                        "expected spawns to complete within {MAX_LOAD_TICKS} ticks; \
                         map errors: {map_errors:?}"
                    );
                });

                ctx.then("NPCs are spawned for SpawnPoints", |state| {
                    assert_eq!(
                        state.npc_count(),
                        EXPECTED_SPAWN_POINT_COUNT,
                        "expected {EXPECTED_SPAWN_POINT_COUNT} NPC(s)"
                    );
                });

                ctx.then("NPC has UnitType component", |state| {
                    assert!(state.npc_has_unit_type(), "NPC should have UnitType");
                });

                ctx.then("NPC UnitType matches enemy_type mapping", |state| {
                    // enemy_type 7 maps to Elite (Baddie with high meanness).
                    assert!(
                        state.npc_is_baddie(),
                        "enemy_type {EXPECTED_ENEMY_TYPE} should map to Baddie"
                    );
                });

                ctx.then("NPC has DdlogId for DBSP sync", |state| {
                    assert!(state.npc_has_ddlog_id(), "NPC should have DdlogId");
                });

                ctx.then("respawning SpawnPoint is NOT marked consumed", |state| {
                    assert!(
                        state.respawning_spawn_point_not_consumed(),
                        "SpawnPoint with respawn=true should not be consumed"
                    );
                });

                ctx.then("SpawnPoint enemy_type matches fixture", |state| {
                    assert_eq!(
                        state.spawn_point_enemy_type(),
                        Some(EXPECTED_ENEMY_TYPE),
                        "SpawnPoint enemy_type should be {EXPECTED_ENEMY_TYPE}"
                    );
                });

                ctx.then("all spawned entity IDs are unique", |state| {
                    assert!(
                        state.all_ids_unique(),
                        "all DdlogId values should be unique"
                    );
                });

                ctx.then("no map errors are emitted", |state| {
                    assert!(state.captured_map_errors().is_empty());
                });
            });
        },
    ));
}
