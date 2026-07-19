//! Map integration plugin that wires Tiled maps into Lille.
//!
//! `LilleMapPlugin` owns the "load the authored map into ECS" entry point and
//! the translation of authored annotations into engine components:
//!
//! - It registers `bevy_ecs_tiled::TiledPlugin` so `.tmx` assets can load.
//! - It spawns a root entity with a `TiledMap` component, which triggers the
//!   `bevy_ecs_tiled` spawn pipeline (layers, tilemaps, etc).
//! - It attaches `Block` components to tiles marked `Collidable` so they
//!   participate in DBSP physics.
//!
//! The DBSP circuit remains the sole source of truth for any inferred behaviour
//! in the game world; this module translates authored data into typed
//! components and feeds them into DBSP.

mod lifecycle;
pub mod spawn;
mod translate;

pub use spawn::{spawn_actors_at_spawn_points, NpcIdCounter};
pub use translate::attach_collision_blocks;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledMapAsset, TiledPlugin};

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
    /// Attempted to load a second map while one is already active.
    DuplicateMapAttempted {
        /// Asset-server path of the map that was requested.
        requested_path: String,
        /// Asset-server path of the map currently loaded.
        active_path: String,
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

impl AsRef<str> for MapAssetPath {
    fn as_ref(&self) -> &str {
        self.as_str()
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
}

impl Default for LilleMapSettings {
    fn default() -> Self {
        Self {
            primary_map: MapAssetPath::default(),
            should_spawn_primary_map: true,
        }
    }
}

/// Marker set by Tiled to flag collidable tiles or objects.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct Collidable;

/// Slope metadata authored in Tiled for sloped terrain tiles.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq)]
#[reflect(Component, Default)]
pub struct SlopeProperties {
    /// Gradient of the slope along the X axis.
    pub grad_x: f32,
    /// Gradient of the slope along the Y axis.
    pub grad_y: f32,
}

/// Marker describing where the player should spawn.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct PlayerSpawn;

/// Metadata for NPC spawn points authored in Tiled.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct SpawnPoint {
    /// Identifier or index that the spawn system can map to a unit archetype.
    pub enemy_type: u32,
    /// Whether the spawn point should respawn after use.
    pub respawn: bool,
}

/// Marker indicating that this entity represents the player character.
///
/// Applied to the spawned player entity to distinguish it from NPCs and
/// to enable player-specific queries.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct Player;

/// Marker indicating that this `PlayerSpawn` point has been consumed.
///
/// Ensures idempotent spawning: the spawn system skips entities with this
/// marker, making it safe to run multiple times or on map reloads.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct PlayerSpawnConsumed;

/// Marker indicating that this `SpawnPoint` has spawned its actor.
///
/// For non-respawning spawn points, this prevents duplicate spawning.
/// Respawning spawn points will have different logic in later phases.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct SpawnPointConsumed;

/// Marker for entities spawned by the map spawn system.
///
/// Allows queries to identify map-spawned actors versus programmatically
/// created entities.
#[derive(Component, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component, Default)]
pub struct MapSpawned;

/// Event to request unloading the currently active primary map.
///
/// When triggered, the map unload system will:
/// 1. Despawn the `PrimaryTiledMap` entity and all its children
/// 2. Despawn all `MapSpawned` entities (player and NPCs)
/// 3. Reset `PrimaryMapAssetTracking` state
/// 4. Allow a new map to be loaded
#[derive(Event, Debug, Clone, Default)]
pub struct UnloadPrimaryMap;

/// Event emitted when the primary map has been fully unloaded.
///
/// Systems that depend on map state can observe this event to know
/// when it is safe to load a new map or perform cleanup.
#[derive(Event, Debug, Clone, Default)]
pub struct PrimaryMapUnloaded;

/// Marker component for the root entity of the primary loaded map.
///
/// Used internally to track the currently loaded map entity. Tests can spawn
/// entities with this marker to simulate an existing map without loading assets.
#[derive(Component, Debug, Default)]
pub struct PrimaryTiledMap;

#[derive(Resource, Default)]
struct LilleMapPluginInstalled;

/// Resource tracking the primary map asset loading state.
///
/// This resource persists the asset handle and path so that load failures
/// can be reported even if the map entity is despawned during error handling.
#[derive(Resource, Debug, Default)]
pub struct PrimaryMapAssetTracking {
    /// Asset-server path of the currently loaded or loading map.
    pub asset_path: Option<String>,
    /// Strong handle to the map asset, kept alive during loading.
    pub handle: Option<Handle<TiledMapAsset>>,
    /// Whether loading has completed (successfully or with failure).
    pub has_finalised: bool,
}

/// Bevy plugin exposing Tiled map support for Lille.
///
/// The plugin is safe to add multiple times: it guarantees `TiledPlugin` is
/// present, and installs Lille-specific systems only once.
#[derive(Debug)]
pub struct LilleMapPlugin;

impl Plugin for LilleMapPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TiledPlugin>() {
            app.add_plugins(TiledPlugin::default());
        }

        if app.world().contains_resource::<LilleMapPluginInstalled>() {
            return;
        }

        app.insert_resource(LilleMapPluginInstalled);
        app.register_type::<Collidable>()
            .register_type::<SlopeProperties>()
            .register_type::<PlayerSpawn>()
            .register_type::<SpawnPoint>()
            .register_type::<Player>()
            .register_type::<PlayerSpawnConsumed>()
            .register_type::<SpawnPointConsumed>()
            .register_type::<MapSpawned>();
        app.add_observer(lifecycle::log_map_error);
        app.add_observer(lifecycle::handle_unload_primary_map);
        app.add_observer(lifecycle::log_map_unloaded);
        app.init_resource::<LilleMapSettings>();
        app.init_resource::<PrimaryMapAssetTracking>();
        app.init_resource::<NpcIdCounter>();
        lifecycle::try_spawn_primary_map_on_build(app);
        app.add_systems(PostStartup, lifecycle::spawn_primary_map_if_enabled);
        app.add_systems(
            Update,
            (
                lifecycle::monitor_primary_map_load_state,
                translate::attach_collision_blocks,
                spawn::spawn_actors_at_spawn_points,
            ),
        );
    }

    fn is_unique(&self) -> bool {
        false
    }
}
