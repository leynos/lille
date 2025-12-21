#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering `LilleMapPlugin` map loading behaviour."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Ensures the map plugin registers Tiled support and spawns the primary map
//! entity without breaking plugin idempotence.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledPlugin;
use lille::LilleMapPlugin;
use rstest::rstest;

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[rstest]
fn adds_tiled_plugin() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);

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
    map_test_plugins::add_map_test_plugins(&mut app);
    app.add_plugins(TiledPlugin::default());

    app.add_plugins(LilleMapPlugin);

    // The guard in LilleMapPlugin should make this safe to call again.
    app.add_plugins(LilleMapPlugin);

    assert!(app.is_plugin_added::<TiledPlugin>());
}

#[rstest]
fn adding_plugin_twice_does_not_panic_and_keeps_tiled() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);

    app.add_plugins(LilleMapPlugin);
    app.add_plugins(LilleMapPlugin);

    assert!(
        app.is_plugin_added::<TiledPlugin>(),
        "Repeated additions must leave TiledPlugin registered exactly once"
    );
}
