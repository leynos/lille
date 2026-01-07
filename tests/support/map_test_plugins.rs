#![cfg(feature = "test-support")]
// The `captured_errors` function is used by some test binaries (map_lifecycle,
// map_lifecycle_rspec) but not others. Clippy's allow_attributes lint conflicts
// with the dead_code warning in binaries that don't use it.
#![allow(
    clippy::allow_attributes,
    reason = "Required for dead_code allowance on shared helpers."
)]
//! Shared test harness helpers for map-related integration tests.
//!
//! These tests must run under `cargo test --all-features` (and `cargo llvm-cov
//! --all-features`), which enables Bevy rendering and Tiled rendering. Bevy's
//! renderer initializes a process-global empty bind group layout; multiple
//! render devices in a single test binary will panic.
//!
//! To keep tests stable:
//! - Any integration test binary that calls `app.update()` should contain only
//!   a single test function.
//! - Other tests in the same binary may still build apps, but should avoid
//!   ticking them.

use bevy::ecs::prelude::On;
use bevy::prelude::*;
use lille::map::LilleMapError;
use rstest::fixture;

#[derive(Resource, Debug, Default)]
struct MapErrorCaptureInstalled;

/// Captures `LilleMapError` events emitted by the map plugin.
///
/// Tests can assert on `CapturedMapErrors.0` to validate both happy and unhappy
/// paths without relying on logs.
#[derive(Resource, Default, Debug)]
pub struct CapturedMapErrors(pub Vec<LilleMapError>);

#[expect(
    clippy::needless_pass_by_value,
    reason = "Observer systems must accept On<T> by value for Events V2."
)]
fn record_map_error(event: On<LilleMapError>, mut captured: ResMut<CapturedMapErrors>) {
    captured.0.push(event.event().clone());
}

/// Installs `CapturedMapErrors` and a corresponding observer into the provided
/// `App`.
///
/// This is idempotent to avoid accidental double-registration in tests.
pub fn install_map_error_capture(app: &mut App) {
    if app.world().contains_resource::<MapErrorCaptureInstalled>() {
        return;
    }

    app.insert_resource(MapErrorCaptureInstalled);
    app.insert_resource(CapturedMapErrors::default());
    app.world_mut().add_observer(record_map_error);
}

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
                    // Avoid initializing WGPU / a render device in CI.
                    // This keeps tests runnable on hosts without a GPU (or
                    // graphics drivers), while still registering render-typed
                    // assets and schedules.
                    backends: None,
                    ..default()
                }),
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>(),
    );

    // `bevy_ecs_tiled` loads tile images; DefaultPlugins already registers the
    // required render-typed assets even when we keep WGPU headless.
}

/// Creates a new `App` configured for map plugin tests.
#[fixture]
pub fn map_test_app() -> App {
    let mut app = App::new();
    add_map_test_plugins(&mut app);
    install_map_error_capture(&mut app);
    app
}

/// Returns errors captured by the map error capture observer.
///
/// Requires `install_map_error_capture` to have been called on the app.
#[allow(
    dead_code,
    reason = "Used by map_lifecycle tests but not all test binaries."
)]
pub fn captured_errors(app: &App) -> Vec<LilleMapError> {
    app.world()
        .get_resource::<CapturedMapErrors>()
        .map(|e| e.0.clone())
        .unwrap_or_default()
}
