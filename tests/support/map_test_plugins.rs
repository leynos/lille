#![cfg(feature = "test-support")]
//! Shared test harness helpers for map-related integration tests.
//!
//! These tests must run under `cargo test --all-features` (and `cargo llvm-cov
//! --all-features`), which enables Bevy rendering and Tiled rendering. Bevy's
//! renderer initialises a process-global empty bind group layout; multiple
//! render devices in a single test binary will panic.
//!
//! To keep tests stable:
//! - Any integration test binary that calls `app.update()` should contain only
//!   a single test function.
//! - Other tests in the same binary may still build apps, but should avoid
//!   ticking them.

use bevy::prelude::*;

/// Adds the minimal set of plugins required to:
/// - run Bevy schedules, and
/// - use Bevy's asset pipeline, and
/// - support `bevy_ecs_tiled` (including its render feature when enabled).
pub fn add_map_test_plugins(app: &mut App) {
    use bevy::log::LogPlugin;
    use bevy::render::settings::WgpuSettings;
    use bevy::render::RenderPlugin;
    use bevy::window::{ExitCondition, WindowPlugin};

    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<LogPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                render_creation: bevy::render::settings::RenderCreation::Automatic(WgpuSettings {
                    force_fallback_adapter: true,
                    ..default()
                }),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>(),
    );

    // `bevy_ecs_tiled` loads tile images, so ensure image asset types exist.
    app.init_asset::<Image>();
    app.init_asset::<TextureAtlasLayout>();
}
