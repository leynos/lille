#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for `LilleMapPlugin` using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Confirms the map plugin leaves the DBSP circuit authoritative when no maps
//! are loaded.

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

use std::sync::{Arc, Mutex, MutexGuard};

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledLayer, TiledMap};
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath, PRIMARY_ISOMETRIC_MAP_PATH};
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

fn add_map_test_plugins(app: &mut App) {
    use bevy::log::LogPlugin;
    use bevy::render::settings::{RenderCreation, WgpuSettings};
    use bevy::render::RenderPlugin;
    use bevy::window::{ExitCondition, WindowPlugin};

    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<LogPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: None,
                    ..default()
                }),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>(),
    );
    app.init_asset::<Image>();
    app.init_asset::<TextureAtlasLayout>();
}

impl MapPluginFixture {
    fn bootstrap() -> Self {
        Self::bootstrap_with_settings(None)
    }

    fn bootstrap_with_settings(settings: Option<LilleMapSettings>) -> Self {
        let mut app = App::new();
        add_map_test_plugins(&mut app);
        app.add_plugins(DbspPlugin);

        if let Some(configured_settings) = settings {
            app.insert_resource(configured_settings);
        }

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
        // Capture panics to log clearer payloads during rspec runs, then rethrow so the harness still fails after any teardown.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut app = self.app_guard();
            if app.world().get_resource::<PluginsFinalised>().is_none() {
                app.finish();
                app.cleanup();
                app.insert_resource(PluginsFinalised);
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
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

    fn tick_until_layers_loaded(&self, max_ticks: usize) {
        for _ in 0..max_ticks {
            self.tick();
            if self.tiled_layer_count() > 0 {
                return;
            }
        }
    }

    fn tick_until_map_error(&self, max_ticks: usize) {
        for _ in 0..max_ticks {
            self.tick();
            if !self.captured_map_errors().is_empty() {
                return;
            }
        }
    }
}

#[test]
fn map_plugin_leaves_dbsp_authoritative_without_maps() {
    let fixture = MapPluginFixture::bootstrap_with_settings(Some(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
        should_bootstrap_camera: false,
    }));

    run_serial(&rspec::given(
        "LilleMapPlugin runs with map spawning disabled",
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
    let fixture = MapPluginFixture::bootstrap_with_settings(Some(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
        should_bootstrap_camera: false,
    }));

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

#[test]
fn map_plugin_loads_primary_map_hierarchy_without_touching_dbsp() {
    let fixture = MapPluginFixture::bootstrap();

    run_serial(&rspec::given(
        "LilleMapPlugin spawns and loads the primary map asset",
        fixture,
        |scenario: &mut Scenario<MapPluginFixture>| {
            scenario.when("the app ticks until map layers appear", |ctx| {
                ctx.before_each(|state| {
                    state.tick_until_layers_loaded(50);
                });

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
            });
        },
    ));
}

#[test]
fn map_plugin_reports_missing_primary_map_and_does_not_panic() {
    let fixture = MapPluginFixture::bootstrap_with_settings(Some(LilleMapSettings {
        primary_map: MapAssetPath::from("maps/does-not-exist.tmx"),
        should_spawn_primary_map: true,
        should_bootstrap_camera: false,
    }));

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
                assert!(state.tiled_map_count() <= 1);
            });
        },
    ));
}
