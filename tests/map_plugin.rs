#![cfg_attr(
    feature = "render",
    doc = "Tests validating the Lille map plugin's Bevy integration."
)]
#![cfg_attr(
    not(feature = "render"),
    doc = "Map plugin tests require the `render` feature."
)]
#![cfg(feature = "render")]
//! Verifies that `LilleMapPlugin` wires the Tiled asset pipeline without
//! overriding player-authored behaviours already expressed through DBSP.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{TiledMap, TiledMapPlugin as ExternalTiledPlugin};
use lille::LilleMapPlugin;
use rstest::rstest;
use std::sync::{Arc, Mutex, PoisonError};

#[rstest]
fn registers_tiled_plugin() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    assert!(
        app.world.get_resource::<Assets<TiledMap>>().is_none(),
        "precondition: TiledMap assets should not exist"
    );

    app.add_plugins(LilleMapPlugin);

    assert!(
        app.world.get_resource::<Assets<TiledMap>>().is_some(),
        "map plugin must register the Tiled asset pipeline"
    );
}

#[test]
fn avoids_duplicate_asset_initialisation() {
    rspec::run(&rspec::given(
        "a headless Bevy app",
        MapHarness::default(),
        |scenario| {
            scenario.before_each(|env| {
                env.reset();
                env.add_tiled_plugin_directly();
            });

            scenario.when(
                "LilleMapPlugin loads even though the upstream plugin is already present",
                |phase| {
                    phase.before_each(|env| {
                        env.add_map_plugin();
                        env.add_map_plugin();
                    });

                    phase.then("the plugin stays registered exactly once", |env| {
                        env.assert_map_plugin_ready();
                    });
                },
            );
        },
    ));
}

#[derive(Clone, Debug)]
struct MapHarness {
    app: Arc<Mutex<App>>,
}

impl Default for MapHarness {
    fn default() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        Self {
            app: Arc::new(Mutex::new(app)),
        }
    }
}

impl MapHarness {
    fn with_app<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        let mut guard = self.app.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut guard)
    }

    fn reset(&self) {
        self.with_app(|app| {
            let mut fresh = App::new();
            fresh.add_plugins(MinimalPlugins);
            *app = fresh;
        });
    }

    fn add_tiled_plugin_directly(&self) {
        self.with_app(|app| {
            app.add_plugins(ExternalTiledPlugin);
        });
    }

    fn add_map_plugin(&self) {
        self.with_app(|app| {
            app.add_plugins(LilleMapPlugin);
        });
    }

    fn assert_map_plugin_ready(&self) {
        self.with_app(|app| {
            assert!(
                app.is_plugin_added::<ExternalTiledPlugin>(),
                "map plugin should remain registered"
            );
            assert!(
                app.world.get_resource::<Assets<TiledMap>>().is_some(),
                "assets resource must remain initialised"
            );
        });
    }
}
