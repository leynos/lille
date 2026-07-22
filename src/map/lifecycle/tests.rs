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
// Windows-style separators must also be rejected as parent traversal.
#[case::windows_parent_traversal("maps\\..\\secrets.tmx")]
#[case::leading_parent("../secrets.tmx")]
#[case::bare_parent("..")]
#[case::nested_parent("maps/tiles/../../secrets.tmx")]
// A `..` component reached via either separator style must be rejected.
#[case::mixed_separator_parent("maps/tiles\\../secrets.tmx")]
// Rooted paths in any platform form must be rejected, not just Unix `/`.
#[case::backslash_root("\\secret.tmx")]
#[case::windows_drive_absolute("C:\\maps\\primary.tmx")]
#[case::unc_path("\\\\server\\share\\primary.tmx")]
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
#[case::nested("levels/act1/room.tmx")]
// `..` embedded in a directory name is not a standalone component.
#[case::dots_in_dir("map..data/level.tmx")]
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
struct InvalidPathObserved(Option<String>);

#[rstest]
fn build_spawn_rejects_invalid_path_without_spawning() {
    let mut app = app_with_settings(true, "/absolute/path.tmx");
    app.init_resource::<InvalidPathObserved>();
    // Observe the rejection directly: `try_spawn_primary_map_on_build`
    // returns early both for an invalid path and for a missing
    // `AssetServer`, so the absence of a map entity alone cannot prove the
    // path was rejected. Capturing the offending path pins down not just
    // that a rejection fired but that it named the configured path.
    app.world_mut().add_observer(
        |event: bevy::ecs::prelude::On<LilleMapError>,
         mut observed: ResMut<InvalidPathObserved>| {
            if let LilleMapError::InvalidPrimaryMapAssetPath { path } = event.event() {
                observed.0 = Some(path.clone());
            }
        },
    );

    try_spawn_primary_map_on_build(&mut app);

    assert_eq!(
        app.world().resource::<InvalidPathObserved>().0.as_deref(),
        Some("/absolute/path.tmx"),
        "expected InvalidPrimaryMapAssetPath to be triggered for the configured path"
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
