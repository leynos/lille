//! Map integration plugin that wires Tiled maps into Lille.
//!
//! `LilleMapPlugin` owns the “load the authored map into ECS” entry point.
//! This is deliberately limited to asset and hierarchy concerns:
//!
//! - It registers `bevy_ecs_tiled::TiledPlugin` so `.tmx` assets can load.
//! - It spawns a root entity with a `TiledMap` component, which triggers the
//!   `bevy_ecs_tiled` spawn pipeline (layers, tilemaps, etc).
//!
//! Importantly, this module must not infer gameplay or physics state. The DBSP
//! circuit remains the sole source of truth for any inferred behaviour in the
//! game world; map loading only exposes authored data so later tasks can
//! translate it into typed components and feed it into DBSP.

use bevy::asset::RecursiveDependencyLoadState;
use bevy::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_ecs_tiled::prelude::{TiledMap, TiledMapAsset, TiledPlugin};
use log::error;

/// Default Tiled map asset path for the “primary” isometric map.
pub const PRIMARY_ISOMETRIC_MAP_PATH: &str = "maps/primary-isometric.tmx";

/// Errors emitted by the map plugin when it cannot load the requested map.
#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub enum LilleMapError {
    /// The configured path was invalid for filesystem-backed assets.
    InvalidPrimaryMapAssetPath {
        /// Asset-server path configured for the primary map.
        path: String,
    },
    /// The primary map asset failed to load.
    PrimaryMapLoadFailed {
        /// Asset-server path configured for the primary map.
        path: String,
        /// Human-readable detail describing why the load failed.
        detail: String,
    },
}

/// Newtype representing a Bevy asset-server path (relative to the asset root).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapAssetPath(String);

impl MapAssetPath {
    /// Creates a new asset path.
    ///
    /// The path must be relative to the Bevy asset root.
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// Borrows the underlying asset path string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MapAssetPath {
    fn from(path: &str) -> Self {
        Self::new(path)
    }
}

impl Default for MapAssetPath {
    fn default() -> Self {
        Self::new(PRIMARY_ISOMETRIC_MAP_PATH)
    }
}

/// Runtime configuration for map loading.
#[derive(Resource, Clone, Debug)]
pub struct LilleMapSettings {
    /// Selected `.tmx` file to load as the primary map.
    pub primary_map: MapAssetPath,
    /// When true, the plugin spawns the primary map in `Startup`.
    pub should_spawn_primary_map: bool,
    /// When true, the plugin spawns a minimal `Camera2d` if none exists.
    pub should_bootstrap_camera: bool,
}

impl Default for LilleMapSettings {
    fn default() -> Self {
        Self {
            primary_map: MapAssetPath::default(),
            should_spawn_primary_map: true,
            should_bootstrap_camera: true,
        }
    }
}

#[derive(Component, Debug)]
struct PrimaryTiledMap;

#[cfg(feature = "render")]
#[derive(Component, Debug)]
struct MapBootstrapCamera;

#[derive(Resource, Default)]
struct LilleMapPluginInstalled;

#[derive(Resource, Debug, Default)]
struct PrimaryMapAssetTracking {
    asset_path: Option<String>,
    handle: Option<Handle<TiledMapAsset>>,
    has_finalised: bool,
}

#[derive(Bundle)]
struct PrimaryTiledMapBundle {
    name: Name,
    marker: PrimaryTiledMap,
    map: TiledMap,
    respawn: bevy_ecs_tiled::prelude::RespawnTiledMap,
    storage: bevy_ecs_tiled::prelude::TiledMapStorage,
    layer_z_offset: bevy_ecs_tiled::prelude::TiledMapLayerZOffset,
    image_repeat_margin: bevy_ecs_tiled::prelude::TiledMapImageRepeatMargin,
    tilemap_render_settings: bevy_ecs_tiled::prelude::TilemapRenderSettings,
    tilemap_anchor: bevy_ecs_tiled::prelude::TilemapAnchor,
    visibility: Visibility,
    transform: Transform,
}

impl PrimaryTiledMapBundle {
    fn new(handle: Handle<TiledMapAsset>) -> Self {
        Self {
            name: Name::new("PrimaryTiledMap"),
            marker: PrimaryTiledMap,
            map: TiledMap(handle),
            respawn: bevy_ecs_tiled::prelude::RespawnTiledMap,
            storage: bevy_ecs_tiled::prelude::TiledMapStorage::default(),
            layer_z_offset: bevy_ecs_tiled::prelude::TiledMapLayerZOffset::default(),
            image_repeat_margin: bevy_ecs_tiled::prelude::TiledMapImageRepeatMargin::default(),
            tilemap_render_settings: bevy_ecs_tiled::prelude::TilemapRenderSettings::default(),
            tilemap_anchor: bevy_ecs_tiled::prelude::TilemapAnchor::default(),
            visibility: Visibility::default(),
            transform: Transform::default(),
        }
    }
}

#[derive(SystemParam)]
struct PrimaryMapSpawnContext<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    settings: Res<'w, LilleMapSettings>,
    existing_maps: Query<'w, 's, (), With<PrimaryTiledMap>>,
    tracking: ResMut<'w, PrimaryMapAssetTracking>,
}

#[cfg(feature = "render")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters use `Res<T>` by value."
)]
fn bootstrap_camera_if_missing(
    mut commands: Commands,
    settings: Res<LilleMapSettings>,
    cameras: Query<(), With<Camera2d>>,
) {
    if !settings.should_bootstrap_camera || !cameras.is_empty() {
        return;
    }

    commands.spawn((
        Camera2d,
        Name::new("MapBootstrapCamera"),
        MapBootstrapCamera,
    ));
}

fn spawn_primary_map_if_enabled(mut commands: Commands, mut context: PrimaryMapSpawnContext) {
    if !context.settings.should_spawn_primary_map {
        return;
    }

    if !context.existing_maps.is_empty() {
        return;
    }

    let asset_path = context.settings.primary_map.as_str().to_owned();
    if let Err(err) = validate_asset_path(&asset_path) {
        commands.trigger(err);
        return;
    }

    let handle = context.asset_server.load(asset_path.clone());
    context.tracking.asset_path = Some(asset_path.clone());
    context.tracking.handle = Some(handle.clone());
    context.tracking.has_finalised = false;
    commands.spawn(PrimaryTiledMapBundle::new(handle));
}

fn validate_asset_path(asset_path: &str) -> Result<(), LilleMapError> {
    if asset_path.is_empty() || asset_path.starts_with('/') || asset_path.contains("..") {
        return Err(LilleMapError::InvalidPrimaryMapAssetPath {
            path: asset_path.to_owned(),
        });
    }

    Ok(())
}

fn try_spawn_primary_map_on_build(app: &mut App) {
    let world = app.world_mut();

    let (should_spawn_primary_map, asset_path) =
        world
            .get_resource::<LilleMapSettings>()
            .map_or((false, String::new()), |settings| {
                (
                    settings.should_spawn_primary_map,
                    settings.primary_map.as_str().to_owned(),
                )
            });

    if !should_spawn_primary_map {
        return;
    }

    let mut existing_maps = world.query_filtered::<Entity, With<PrimaryTiledMap>>();
    if existing_maps.iter(world).next().is_some() {
        return;
    }

    if let Err(err) = validate_asset_path(&asset_path) {
        world.trigger(err);
        return;
    }

    let Some(asset_server) = world.get_resource::<AssetServer>() else {
        return;
    };

    let handle = asset_server.load(asset_path.clone());
    {
        let mut tracking = world.resource_mut::<PrimaryMapAssetTracking>();
        tracking.asset_path = Some(asset_path.clone());
        tracking.handle = Some(handle.clone());
        tracking.has_finalised = false;
    }
    world.spawn(PrimaryTiledMapBundle::new(handle));
}

/// Bevy plugin exposing Tiled map support for Lille.
///
/// The plugin is safe to add multiple times: it guarantees `TiledPlugin` is
/// present, and installs Lille-specific systems only once.
#[derive(Debug)]
pub struct LilleMapPlugin;

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
fn log_map_error(event: bevy::ecs::prelude::On<LilleMapError>) {
    error!("map error: {:?}", event.event());
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters use `Res<T>` by value."
)]
fn monitor_primary_map_load_state(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tracking: ResMut<PrimaryMapAssetTracking>,
) {
    if tracking.has_finalised {
        return;
    }

    let Some(handle) = tracking.handle.clone() else {
        return;
    };

    match asset_server.recursive_dependency_load_state(handle.id()) {
        RecursiveDependencyLoadState::Loaded => {
            tracking.has_finalised = true;
        }
        RecursiveDependencyLoadState::Failed(error) => {
            commands.trigger(LilleMapError::PrimaryMapLoadFailed {
                path: tracking.asset_path.clone().unwrap_or_default(),
                detail: error.to_string(),
            });
            tracking.has_finalised = true;
        }
        RecursiveDependencyLoadState::NotLoaded | RecursiveDependencyLoadState::Loading => {}
    }
}

impl Plugin for LilleMapPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TiledPlugin>() {
            app.add_plugins(TiledPlugin::default());
        }

        app.add_observer(log_map_error);

        if app.world().contains_resource::<LilleMapPluginInstalled>() {
            return;
        }

        app.insert_resource(LilleMapPluginInstalled);
        app.init_resource::<LilleMapSettings>();
        app.init_resource::<PrimaryMapAssetTracking>();
        try_spawn_primary_map_on_build(app);
        #[cfg(feature = "render")]
        app.add_systems(Startup, bootstrap_camera_if_missing);
        app.add_systems(PostStartup, spawn_primary_map_if_enabled);
        app.add_systems(Update, monitor_primary_map_load_state);
    }

    fn is_unique(&self) -> bool {
        false
    }
}
