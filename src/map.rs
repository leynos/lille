//! Map integration plugin that wires Tiled maps into Lille.
//!
//! This skeleton plugin registers `bevy_ecs_tiled::TiledPlugin` so map assets
//! and events are available to the engine. The `map` feature currently depends
//! on the `render` feature, so the plugin assumes the render stack is compiled
//! in even though it does not itself add the renderer to the `App`. It
//! intentionally avoids spawning geometry or entities so the DBSP circuit
//! remains the source of truth for inferred behaviour; later tasks in the
//! roadmap will layer map loading and component translation on top of this
//! entry point.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::TiledPlugin;

/// Bevy plugin exposing Tiled map support for Lille.
///
/// The plugin is idempotent: if `TiledPlugin` is already present (for example
/// in external tooling), it will not be re-added. This keeps scheduling stable
/// while allowing downstream crates to compose their own plugin stacks.
#[derive(Debug)]
pub struct LilleMapPlugin;

impl Plugin for LilleMapPlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<TiledPlugin>() {
            return;
        }

        app.add_plugins(TiledPlugin::default());
    }

    fn is_unique(&self) -> bool {
        false
    }
}
