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
// A drive letter without a following separator is drive-relative, not
// drive-absolute, so it is not a rooted path.
#[case::bare_drive_letter("C:")]
#[case::drive_relative("C:maps/primary.tmx")]
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

/// Bounded generator over representative components and both separator styles.
/// Every rooted path or path containing a standalone `..` component must be
/// rejected, while relative paths whose components merely contain `..` as a
/// substring must be accepted.
#[rstest]
fn validate_asset_path_component_matrix() {
    // Some components embed `..` as a substring but are not the `..` component.
    let safe_components = ["maps", "level.tmx", "primary..backup", "a..b", "sub"];

    for sep in ['/', '\\'] {
        let sep_str = sep.to_string();

        let relative = safe_components.join(&sep_str);
        assert!(
            validate_asset_path(&relative).is_ok(),
            "relative path {relative:?} with only substring dots must be accepted"
        );

        // Inserting a standalone `..` component at any position must be rejected,
        // regardless of which separator style delimits it.
        for insert_at in 0..=safe_components.len() {
            let mut parts = safe_components.to_vec();
            parts.insert(insert_at, "..");
            let traversal = parts.join(&sep_str);
            assert!(
                matches!(
                    validate_asset_path(&traversal),
                    Err(LilleMapError::InvalidPrimaryMapAssetPath { .. })
                ),
                "path {traversal:?} with a standalone `..` component must be rejected"
            );
        }
    }

    // Rooted paths in every platform form are rejected regardless of contents.
    for path in [
        "/maps/level.tmx",
        "\\maps\\level.tmx",
        "C:\\maps\\level.tmx",
        "c:/maps/level.tmx",
        "\\\\server\\share\\level.tmx",
    ] {
        assert!(
            matches!(
                validate_asset_path(path),
                Err(LilleMapError::InvalidPrimaryMapAssetPath { .. })
            ),
            "rooted path {path:?} must be rejected"
        );
    }
}

mod properties {
    //! Property-based coverage supplementing the deterministic matrices above.
    //!
    //! [`validate_asset_path_component_matrix`] enumerates a fixed set of
    //! components and both separator styles exhaustively; this samples arbitrary
    //! strings instead, including the separator and prefix shapes an attacker
    //! would reach for. See `docs/adr-003-bounded-rstest-over-property-testing.md`.

    use super::*;
    use proptest::prelude::*;

    /// The documented contract, restated independently of the implementation:
    /// an asset path is rejected when it is empty, when it is rooted in any
    /// platform form, or when any whole path component is `..`. Everything else
    /// is accepted, including components that merely contain `..` as a
    /// substring and paths with repeated separators (an empty component is not
    /// itself a rejection reason).
    ///
    /// A drive letter counts as rooted only when a separator follows it:
    /// `C:/x` and `C:\x` are drive-absolute, while `C:` and `C:x` are
    /// drive-relative and therefore accepted.
    fn contract_rejects(path: &str) -> bool {
        if path.is_empty() {
            return true;
        }
        let rooted = path.starts_with('/')
            || path.starts_with('\\')
            || matches!(
                path.as_bytes(),
                [drive, b':', separator, ..]
                    if drive.is_ascii_alphabetic() && matches!(separator, b'/' | b'\\')
            );
        rooted || path.split(['/', '\\']).any(|component| component == "..")
    }

    /// Fragments assembled into candidate paths: separators in both styles,
    /// empty components, dot runs, drive and UNC prefixes, and ordinary names.
    fn fragment_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("/".to_owned()),
            Just("\\".to_owned()),
            Just("//".to_owned()),
            Just("..".to_owned()),
            Just(".".to_owned()),
            Just("...".to_owned()),
            Just(String::new()),
            Just("C:".to_owned()),
            Just("\\\\server".to_owned()),
            Just("maps".to_owned()),
            Just("primary..backup".to_owned()),
            Just("level.tmx".to_owned()),
            "[a-zA-Z0-9._\\\\/:-]{0,6}",
        ]
    }

    fn path_strategy() -> impl Strategy<Value = String> {
        prop::collection::vec(fragment_strategy(), 0..6).prop_map(|parts| parts.concat())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        /// `validate_asset_path` agrees with the documented contract on
        /// arbitrary strings.
        #[test]
        fn validate_asset_path_agrees_with_the_documented_contract(path in path_strategy()) {
            let rejected = matches!(
                validate_asset_path(&path),
                Err(LilleMapError::InvalidPrimaryMapAssetPath { .. })
            );
            prop_assert_eq!(
                rejected,
                contract_rejects(&path),
                "validate_asset_path disagreed with the contract for {:?}",
                path
            );
        }
    }
}
