//! Lifecycle systems for the primary Tiled map.
//!
//! Houses the spawn, validation, monitoring, and unload systems that
//! `LilleMapPlugin` registers. Splitting these from the plugin and component
//! definitions keeps each module focused and within the repository's module
//! size budget.

use bevy::asset::RecursiveDependencyLoadState;
use bevy::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_ecs_tiled::prelude::{TiledMap, TiledMapAsset};
use log::error;

use super::{
    LilleMapError, LilleMapSettings, MapSpawned, PrimaryMapAssetTracking, PrimaryMapUnloaded,
    PrimaryTiledMap, UnloadPrimaryMap,
};

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
            map: TiledMap(handle),
            ..Self::default()
        }
    }
}

impl Default for PrimaryTiledMapBundle {
    fn default() -> Self {
        Self {
            name: Name::new("PrimaryTiledMap"),
            marker: PrimaryTiledMap,
            map: TiledMap(Handle::default()),
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
pub(super) struct PrimaryMapSpawnContext<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    settings: Res<'w, LilleMapSettings>,
    existing_maps: Query<'w, 's, (), With<PrimaryTiledMap>>,
    tracking: ResMut<'w, PrimaryMapAssetTracking>,
}

pub(super) fn spawn_primary_map_if_enabled(
    mut commands: Commands,
    mut context: PrimaryMapSpawnContext,
) {
    if !context.settings.should_spawn_primary_map {
        return;
    }

    // If tracking already has an asset path, we've already committed to loading a map.
    // This is normal operation after the first tick - just return silently.
    if context.tracking.asset_path.is_some() {
        return;
    }

    // If a map entity exists but tracking doesn't have a path, something external
    // spawned a map. Emit an error since this violates single-map semantics.
    if !context.existing_maps.is_empty() {
        let requested_path = context.settings.primary_map.as_str().to_owned();
        let active_path = "[external]".to_owned();

        log::warn!(
            "Attempted to load map '{requested_path}' while an external map is already active; \
             ignoring request"
        );

        commands.trigger(LilleMapError::DuplicateMapAttempted {
            requested_path,
            active_path,
        });
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
    if asset_path.is_empty() {
        return Err(LilleMapError::InvalidPrimaryMapAssetPath {
            path: asset_path.to_owned(),
        });
    }

    // Reject rooted paths in any form the asset loader might resolve outside
    // the asset root: Unix-absolute (`/...`), Windows backslash-root and UNC
    // (`\...`, `\\server\share\...`), and drive-absolute (`C:\...`, `C:/...`).
    if is_rooted_path(asset_path) {
        return Err(LilleMapError::InvalidPrimaryMapAssetPath {
            path: asset_path.to_owned(),
        });
    }

    // Reject parent-directory traversal, but only when `..` is a whole path
    // component. A substring check would wrongly reject legitimate filenames
    // such as `maps/primary..backup.tmx`. Split on both slash forms so a
    // Windows-style separator (`maps\..\secret.tmx`) cannot smuggle a `..`
    // component past this check.
    if asset_path
        .split(['/', '\\'])
        .any(|component| component == "..")
    {
        return Err(LilleMapError::InvalidPrimaryMapAssetPath {
            path: asset_path.to_owned(),
        });
    }

    Ok(())
}

/// Reports whether `path` is rooted (absolute) on any target platform.
///
/// Covers Unix-absolute (`/...`), Windows backslash-root and UNC
/// (`\...`, `\\server\share`), and drive-letter-absolute paths (`C:\...` or
/// `C:/...`). Asset paths must stay relative to the asset root, so any rooted
/// form is rejected regardless of the host operating system.
fn is_rooted_path(path: &str) -> bool {
    if path.starts_with('/') || path.starts_with('\\') {
        return true;
    }
    // Drive-absolute, e.g. `C:\maps\primary.tmx` or `C:/maps/primary.tmx`.
    let mut chars = path.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(drive), Some(':')) if drive.is_ascii_alphabetic()
    )
}

pub(super) fn try_spawn_primary_map_on_build(app: &mut App) {
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

/// Observer that handles `UnloadPrimaryMap` events by despawning map entities.
///
/// This observer enables safe hot-reload by:
/// 1. Despawning the `PrimaryTiledMap` entity and all children (tiles, layers)
/// 2. Despawning all `MapSpawned` entities (player, NPCs)
/// 3. Resetting `PrimaryMapAssetTracking` to allow new map loads
///
/// # Bevy 0.17 Despawn Behaviour
///
/// In Bevy 0.17+, `despawn()` automatically despawns all descendants via the
/// `ChildOf` relationship. The deprecated `despawn_recursive()` is no longer
/// available on `EntityCommands`. Child entities (tiles, layers from
/// `bevy_ecs_tiled`) are removed when their parent is despawned.
#[expect(
    clippy::too_many_arguments,
    reason = "Bevy observer systems require query parameters; grouping would obscure intent."
)]
pub(super) fn handle_unload_primary_map(
    _event: bevy::ecs::prelude::On<UnloadPrimaryMap>,
    mut commands: Commands,
    map_query: Query<Entity, With<PrimaryTiledMap>>,
    spawned_query: Query<Entity, With<MapSpawned>>,
    mut tracking: ResMut<PrimaryMapAssetTracking>,
) {
    let mut unloaded_any = false;

    // Note: Bevy 0.17's despawn() handles ChildOf relationships automatically,
    // removing all descendant entities (tiles, layers) when the root is despawned.
    for map_entity in &map_query {
        commands.entity(map_entity).despawn();
        unloaded_any = true;
        log::info!("Unloaded primary map entity {map_entity:?}");
    }

    // Note: Bevy 0.17's despawn() handles ChildOf relationships automatically,
    // removing any child entities (sprites, effects) when the actor is despawned.
    for spawned_entity in &spawned_query {
        commands.entity(spawned_entity).despawn();
        log::debug!("Despawned map-spawned entity {spawned_entity:?}");
    }

    tracking.asset_path = None;
    tracking.handle = None;
    tracking.has_finalised = false;

    if unloaded_any {
        commands.trigger(PrimaryMapUnloaded);
    }
}

pub(super) fn log_map_unloaded(_event: bevy::ecs::prelude::On<PrimaryMapUnloaded>) {
    log::info!("Primary map unloaded successfully");
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
pub(super) fn log_map_error(event: bevy::ecs::prelude::On<LilleMapError>) {
    error!("map error: {:?}", event.event());
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters use `Res<T>` by value."
)]
pub(super) fn monitor_primary_map_load_state(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tracking: ResMut<PrimaryMapAssetTracking>,
) {
    if tracking.has_finalised {
        return;
    }

    let Some(asset_id) = tracking.handle.as_ref().map(bevy::prelude::Handle::id) else {
        return;
    };

    match asset_server.recursive_dependency_load_state(asset_id) {
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

#[cfg(test)]
mod tests;
