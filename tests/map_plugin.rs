#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` map loading behaviour."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Ensures the map plugin registers Tiled support and spawns the primary map
//! entity without breaking plugin idempotence.

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledLayer, TiledMap, TiledPlugin};
use lille::map::{LilleMapError, LilleMapSettings, MapAssetPath, PRIMARY_ISOMETRIC_MAP_PATH};
use lille::LilleMapPlugin;
use rstest::rstest;

#[derive(Resource, Default, Debug)]
struct CapturedMapErrors(pub Vec<LilleMapError>);

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
    app.init_asset::<TextureAtlasLayout>();
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
fn record_map_error(event: On<LilleMapError>, mut captured: ResMut<CapturedMapErrors>) {
    captured.0.push(event.event().clone());
}

#[rstest]
fn adds_tiled_plugin() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);

    app.add_plugins(LilleMapPlugin);

    assert!(
        app.is_plugin_added::<TiledPlugin>(),
        "LilleMapPlugin should add TiledPlugin; if this fails, map support \
         is no longer being initialised and this is a regression."
    );
}

#[rstest]
fn does_not_readd_if_already_present() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    app.add_plugins(TiledPlugin::default());

    app.add_plugins(LilleMapPlugin);

    // The guard in LilleMapPlugin should make this safe to call again.
    app.add_plugins(LilleMapPlugin);

    assert!(app.is_plugin_added::<TiledPlugin>());
}

#[rstest]
fn adding_plugin_twice_does_not_panic_and_keeps_tiled() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);

    app.add_plugins(LilleMapPlugin);
    app.add_plugins(LilleMapPlugin);

    assert!(
        app.is_plugin_added::<TiledPlugin>(),
        "Repeated additions must leave TiledPlugin registered exactly once"
    );
}

#[rstest]
fn spawns_primary_tiled_map_on_startup() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    assert!(
        app.world().get_resource::<AssetServer>().is_some(),
        "expected AssetServer to be available after adding AssetPlugin"
    );
    assert!(
        app.world()
            .resource::<LilleMapSettings>()
            .should_spawn_primary_map,
        "expected primary map spawning to be enabled by default"
    );

    // Startup runs on first update.
    app.update();

    let world = app.world_mut();
    let mut query = world.query::<&TiledMap>();
    let map = query
        .iter(world)
        .next()
        .expect("expected LilleMapPlugin to spawn a TiledMap entity on startup");

    let asset_server = world.resource::<AssetServer>();
    let path = asset_server
        .get_path(map.0.id())
        .expect("expected spawned map handle to have an associated path");
    assert_eq!(
        path.path().to_string_lossy(),
        PRIMARY_ISOMETRIC_MAP_PATH,
        "spawned TiledMap should point at the primary map asset"
    );
}

#[rstest]
fn does_not_spawn_primary_map_when_disabled() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from(PRIMARY_ISOMETRIC_MAP_PATH),
        should_spawn_primary_map: false,
        should_bootstrap_camera: false,
    });

    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    app.update();

    let world = app.world_mut();
    assert!(
        world.query::<&TiledMap>().iter(world).next().is_none(),
        "disabling primary map spawn should leave no TiledMap entities",
    );
}

#[rstest]
fn missing_primary_map_triggers_error_and_map_does_not_load() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from("maps/does-not-exist.tmx"),
        should_spawn_primary_map: true,
        should_bootstrap_camera: false,
    });

    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    let mut load_failed = false;
    for _ in 0..200 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
        if !app.world().resource::<CapturedMapErrors>().0.is_empty() {
            load_failed = true;
            break;
        }
    }

    let world = app.world_mut();
    let captured = world.resource::<CapturedMapErrors>();
    let first = captured
        .0
        .first()
        .expect("expected a map load failure to be captured");
    assert!(
        matches!(first, LilleMapError::PrimaryMapLoadFailed { .. }),
        "expected PrimaryMapLoadFailed error",
    );
    assert!(
        load_failed,
        "expected map load failure to occur within 200 ticks"
    );
}

#[rstest]
fn invalid_primary_map_path_triggers_error() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
    app.insert_resource(LilleMapSettings {
        primary_map: MapAssetPath::from("/not-a-valid-asset-path.tmx"),
        should_spawn_primary_map: true,
        should_bootstrap_camera: false,
    });

    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();
    app.update();

    let captured = app.world().resource::<CapturedMapErrors>();
    let first = captured
        .0
        .first()
        .expect("expected an invalid map asset path error to be captured");

    assert!(
        matches!(first, LilleMapError::InvalidPrimaryMapAssetPath { .. }),
        "expected InvalidPrimaryMapAssetPath error"
    );
}

#[rstest]
fn loads_primary_map_hierarchy_layers_after_some_ticks() {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
    app.add_plugins(LilleMapPlugin);
    app.finish();
    app.cleanup();

    let mut layer_found = false;
    for _ in 0..2_000 {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(1));
        if !app.world().resource::<CapturedMapErrors>().0.is_empty() {
            break;
        }
        let world = app.world_mut();
        if world.query::<&TiledLayer>().iter(world).next().is_some() {
            layer_found = true;
            break;
        }
    }

    let errors = app.world().resource::<CapturedMapErrors>();
    assert!(
        errors.0.is_empty(),
        "expected the primary map to load without emitting LilleMapError events, but observed: {:?}",
        errors.0
    );

    if !layer_found {
        let world = app.world_mut();
        let (map_entity, map_handle) = world
            .query::<(Entity, &TiledMap)>()
            .iter(world)
            .next()
            .expect("expected the primary map root entity to exist");
        let load_state = world
            .resource::<AssetServer>()
            .get_recursive_dependency_load_state(&map_handle.0);
        panic!(
            "expected at least one TiledLayer to spawn for map entity {map_entity:?}, but none \
             appeared; recursive dependency load state: {load_state:?}",
        );
    }
    assert!(
        layer_found,
        "expected at least one TiledLayer to be spawned after loading the primary map",
    );
}
