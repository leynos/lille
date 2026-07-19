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

    if asset_path.starts_with('/') {
        return Err(LilleMapError::InvalidPrimaryMapAssetPath {
            path: asset_path.to_owned(),
        });
    }

    // Reject parent-directory traversal, but only when `..` is a whole path
    // component. A substring check would wrongly reject legitimate filenames
    // such as `maps/primary..backup.tmx`.
    if asset_path.split('/').any(|component| component == "..") {
        return Err(LilleMapError::InvalidPrimaryMapAssetPath {
            path: asset_path.to_owned(),
        });
    }

    Ok(())
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
mod tests {
    //! Tests for the map lifecycle helpers that need no asset backend.
    use bevy::prelude::*;
    use rstest::rstest;

    use super::*;

    fn app_with_settings(should_spawn: bool, path: &str) -> App {
        let mut app = App::new();
        app.insert_resource(LilleMapSettings {
            primary_map: super::super::MapAssetPath::from(path),
            should_spawn_primary_map: should_spawn,
        });
        app.init_resource::<PrimaryMapAssetTracking>();
        app
    }

    #[rstest]
    #[case::empty_path("")]
    #[case::absolute_path("/etc/maps/primary.tmx")]
    #[case::parent_traversal("maps/../secrets.tmx")]
    fn validate_asset_path_rejects_unsafe_paths(#[case] path: &str) {
        let result = validate_asset_path(path);
        assert!(
            matches!(
                result,
                Err(LilleMapError::InvalidPrimaryMapAssetPath { path: ref p }) if p == path
            ),
            "expected InvalidPrimaryMapAssetPath for {path:?}, got {result:?}"
        );
    }

    #[rstest]
    #[case::plain("maps/primary-isometric.tmx")]
    // `..` inside a filename is not a path component, so it must be accepted.
    #[case::dots_in_filename("maps/primary..backup.tmx")]
    fn validate_asset_path_accepts_relative_paths(#[case] path: &str) {
        assert!(
            validate_asset_path(path).is_ok(),
            "expected {path:?} to be accepted"
        );
    }

    #[rstest]
    fn build_spawn_skips_when_disabled() {
        let mut app = app_with_settings(false, "maps/primary-isometric.tmx");
        try_spawn_primary_map_on_build(&mut app);
        let tracking = app.world().resource::<PrimaryMapAssetTracking>();
        assert!(tracking.asset_path.is_none());
    }

    #[rstest]
    fn build_spawn_skips_when_map_already_present() {
        let mut app = app_with_settings(true, "maps/primary-isometric.tmx");
        app.world_mut().spawn(PrimaryTiledMap);
        try_spawn_primary_map_on_build(&mut app);
        let tracking = app.world().resource::<PrimaryMapAssetTracking>();
        assert!(tracking.asset_path.is_none());
    }

    #[derive(Resource, Default)]
    struct InvalidPathObserved(bool);

    #[rstest]
    fn build_spawn_rejects_invalid_path_without_spawning() {
        let mut app = app_with_settings(true, "/absolute/path.tmx");
        app.init_resource::<InvalidPathObserved>();
        // Observe the rejection directly: `try_spawn_primary_map_on_build`
        // returns early both for an invalid path and for a missing
        // `AssetServer`, so the absence of a map entity alone cannot prove the
        // path was rejected. The observer pins down the actual cause.
        app.world_mut().add_observer(
            |event: bevy::ecs::prelude::On<LilleMapError>,
             mut observed: ResMut<InvalidPathObserved>| {
                if matches!(
                    event.event(),
                    LilleMapError::InvalidPrimaryMapAssetPath { .. }
                ) {
                    observed.0 = true;
                }
            },
        );

        try_spawn_primary_map_on_build(&mut app);

        assert!(
            app.world().resource::<InvalidPathObserved>().0,
            "expected InvalidPrimaryMapAssetPath to be triggered"
        );

        let world = app.world_mut();
        let mut maps = world.query_filtered::<Entity, With<PrimaryTiledMap>>();
        assert!(maps.iter(world).next().is_none());
    }

    #[rstest]
    fn build_spawn_skips_without_asset_server() {
        let mut app = app_with_settings(true, "maps/primary-isometric.tmx");
        try_spawn_primary_map_on_build(&mut app);
        let tracking = app.world().resource::<PrimaryMapAssetTracking>();
        assert!(tracking.asset_path.is_none());
    }
}
