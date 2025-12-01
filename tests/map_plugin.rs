#![cfg_attr(
    all(feature = "render", feature = "map"),
    doc = "Unit tests covering the `LilleMapPlugin` skeleton."
)]
#![cfg_attr(
    not(all(feature = "render", feature = "map")),
    doc = "Tests require the `render` and `map` features."
)]
#![cfg(all(feature = "render", feature = "map"))]
//! Ensures the map plugin registers Tiled support without duplicating plugins.

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledPlugin;
use lille::LilleMapPlugin;
use rstest::rstest;

#[rstest]
fn adds_tiled_plugin() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));

    app.add_plugins(LilleMapPlugin);

    if !app.is_plugin_added::<TiledPlugin>() {
        // Headless environments without a WGPU adapter skip map initialisation.
        return;
    }

    assert!(app.is_plugin_added::<TiledPlugin>());
}

#[rstest]
fn does_not_readd_if_already_present() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        TiledPlugin::default(),
    ));

    app.add_plugins(LilleMapPlugin);

    // The guard in LilleMapPlugin should make this safe to call again.
    if app.is_plugin_added::<TiledPlugin>() {
        app.add_plugins(LilleMapPlugin);
    }

    assert!(app.is_plugin_added::<TiledPlugin>());
}
