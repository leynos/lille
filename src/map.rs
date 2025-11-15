//! Lille isometric map plugin scaffolding.
//!
//! `LilleMapPlugin` encapsulates the map asset wiring so the DBSP circuit
//! remains the single authority for inferred world behaviour. The plugin only
//! exposes the data pipeline, allowing DBSP-driven systems to interpret the
//! loaded entities.
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledMapPlugin as TiledPlugin;

/// Registers the `bevy_ecs_tiled` plugin exactly once.
///
/// Add this plugin alongside `DbspPlugin` to ensure the application can load
/// `.tmx` assets authored in Tiled:
///
/// ```no_run
/// use bevy::prelude::*;
/// use lille::{DbspPlugin, LilleMapPlugin};
///
/// App::new()
///     .add_plugins((DbspPlugin, LilleMapPlugin))
///     .run();
/// ```
#[derive(Default, Debug, Clone, Copy)]
pub struct LilleMapPlugin;

impl Plugin for LilleMapPlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<TiledPlugin>() {
            return;
        }

        app.add_plugins(TiledPlugin);
    }
}
