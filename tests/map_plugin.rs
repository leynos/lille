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
use std::ptr;
use std::sync::{Arc, Mutex, PoisonError};

#[rstest]
fn registers_tiled_plugin() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    assert!(
        !app.is_plugin_added::<ExternalTiledPlugin>(),
        "precondition: TiledMapPlugin should not be registered"
    );

    app.add_plugins(LilleMapPlugin);

    assert!(
        app.is_plugin_added::<ExternalTiledPlugin>(),
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
                        env.cache_asset_ptr();
                        env.add_map_plugin();
                        env.add_map_plugin();
                    });

                    phase.then(
                        "the existing asset store remains the source of truth",
                        |env| {
                            let before = env.take_cached_asset_ptr();
                            let after = env.map_asset_ptr();
                            assert_eq!(before, after);
                        },
                    );
                },
            );
        },
    ));
}

#[derive(Clone, Debug)]
struct MapHarness {
    app: Arc<Mutex<App>>,
    cached_asset_ptr: Arc<Mutex<Option<usize>>>,
}

impl Default for MapHarness {
    fn default() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        Self {
            app: Arc::new(Mutex::new(app)),
            cached_asset_ptr: Arc::new(Mutex::new(None)),
        }
    }
}

impl MapHarness {
    fn with_app<R>(&self, f: impl FnOnce(&mut App) -> R) -> R {
        let mut guard = self
            .app
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        f(&mut guard)
    }

    fn reset(&self) {
        self.with_app(|app| {
            let mut fresh = App::new();
            fresh.add_plugins(MinimalPlugins);
            *app = fresh;
        });
        self.cached_asset_ptr
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
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

    fn map_asset_ptr(&self) -> usize {
        self.with_app(|app| {
            let assets = app
                .world
                .get_resource::<Assets<TiledMap>>()
                .unwrap_or_else(|| panic!("expected TiledMap assets"));
            ptr::from_ref(assets) as usize
        })
    }

    fn cache_asset_ptr(&self) {
        let mut guard = self
            .cached_asset_ptr
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        *guard = Some(self.map_asset_ptr());
    }

    fn take_cached_asset_ptr(&self) -> usize {
        self.cached_asset_ptr
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .unwrap_or_else(|| panic!("asset pointer should be cached"))
    }
}
