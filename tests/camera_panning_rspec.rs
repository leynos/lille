#![cfg_attr(
    all(feature = "test-support", feature = "render"),
    doc = "Behavioural tests for camera panning using rust-rspec."
)]
#![cfg_attr(
    not(all(feature = "test-support", feature = "render")),
    doc = "Behavioural tests require `test-support` and `render` features."
)]
#![cfg(all(feature = "test-support", feature = "render"))]
//! Behavioural test: Camera panning responds to WASD and arrow keys.
//!
//! These tests verify that keyboard input correctly moves the camera in the
//! expected direction and that diagonal movement is normalised.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

#[path = "support/map_fixture.rs"]
mod map_fixture;

use std::sync::MutexGuard;

use bevy::input::ButtonInput;
use bevy::prelude::*;
use lille::presentation::{CameraController, CameraSettings};
use lille::PresentationPlugin;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

/// Fixed pan speed for deterministic tests.
const TEST_PAN_SPEED: f32 = 100.0;

/// Fixture for camera panning behavioural tests.
#[derive(Debug, Clone)]
struct CameraPanFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl CameraPanFixture {
    /// Creates a test fixture with the `PresentationPlugin` installed.
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);

        // Insert custom settings before plugin to override defaults.
        app.insert_resource(CameraSettings {
            pan_speed: TEST_PAN_SPEED,
            max_delta_seconds: 1.0, // Allow large deltas in tests
        });

        app.add_plugins(PresentationPlugin);

        Self {
            base: map_fixture::MapPluginFixtureBase::new(app),
        }
    }

    /// Locks the underlying `App` for direct inspection or mutation.
    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        self.base.app_guard()
    }

    /// Advances the application by a single tick.
    fn tick(&self) {
        self.base.tick();
    }

    /// Simulates pressing a key.
    fn press_key(&self, key: KeyCode) {
        let mut app = self.app_guard();
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.press(key);
    }

    /// Simulates releasing a key.
    #[expect(dead_code, reason = "May be used in future tests")]
    fn release_key(&self, key: KeyCode) {
        let mut app = self.app_guard();
        let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keyboard.release(key);
    }

    /// Clears all keyboard input and resets camera to origin.
    fn reset_state(&self) {
        let mut app = self.app_guard();
        // Release all movement keys explicitly
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.release(KeyCode::KeyW);
            keyboard.release(KeyCode::KeyA);
            keyboard.release(KeyCode::KeyS);
            keyboard.release(KeyCode::KeyD);
            keyboard.release(KeyCode::ArrowUp);
            keyboard.release(KeyCode::ArrowDown);
            keyboard.release(KeyCode::ArrowLeft);
            keyboard.release(KeyCode::ArrowRight);
            keyboard.clear();
        }
        // Reset camera to origin
        let world = app.world_mut();
        let mut query = world.query::<(&mut Transform, &CameraController)>();
        for (mut transform, _) in query.iter_mut(world) {
            transform.translation = Vec3::ZERO;
        }
    }

    /// Returns the camera position if a camera exists.
    fn camera_position(&self) -> Option<Vec3> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&Transform, &CameraController)>();
        query
            .iter(world)
            .next()
            .map(|(transform, _)| transform.translation)
    }
}

/// Helper to test that pressing a key moves the camera in the expected direction.
#[expect(
    clippy::too_many_arguments,
    reason = "Test helper with semantically distinct parameters for BDD scenario construction"
)]
fn test_key_movement<F>(
    scenario: &mut Scenario<CameraPanFixture>,
    key: KeyCode,
    when_desc: &'static str,
    then_desc: &'static str,
    assertion: F,
    error_msg: &'static str,
) where
    F: Fn(&Vec3) -> bool + Clone + 'static,
{
    scenario.when(when_desc, move |ctx| {
        ctx.before_each(move |state| {
            state.reset_state();
            state.tick();
            state.press_key(key);
            state.tick();
        });

        let check = assertion.clone();
        ctx.then(then_desc, move |state| {
            let pos = state
                .camera_position()
                .unwrap_or_else(|| panic!("camera should exist"));
            assert!(check(&pos), "{error_msg}, got {pos:?}");
        });
    });
}

#[test]
fn camera_pans_with_wasd_keys() {
    let fixture = CameraPanFixture::bootstrap();

    run_serial(&rspec::given(
        "PresentationPlugin camera responds to WASD input",
        fixture,
        |scenario: &mut Scenario<CameraPanFixture>| {
            scenario.when("the app initialises", |ctx| {
                ctx.before_each(|state| {
                    state.tick(); // Finalise plugins and spawn camera
                });

                ctx.then("camera starts at origin", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    assert!(
                        pos.x.abs() < 0.001 && pos.y.abs() < 0.001,
                        "camera should start near origin, got {pos:?}"
                    );
                });
            });

            test_key_movement(
                scenario,
                KeyCode::KeyW,
                "W key is pressed",
                "camera moves up (positive Y)",
                |pos| pos.y > 0.0,
                "camera Y should increase when W pressed",
            );
            test_key_movement(
                scenario,
                KeyCode::KeyS,
                "S key is pressed",
                "camera moves down (negative Y)",
                |pos| pos.y < 0.0,
                "camera Y should decrease when S pressed",
            );
            test_key_movement(
                scenario,
                KeyCode::KeyA,
                "A key is pressed",
                "camera moves left (negative X)",
                |pos| pos.x < 0.0,
                "camera X should decrease when A pressed",
            );
            test_key_movement(
                scenario,
                KeyCode::KeyD,
                "D key is pressed",
                "camera moves right (positive X)",
                |pos| pos.x > 0.0,
                "camera X should increase when D pressed",
            );
        },
    ));
}

#[test]
fn camera_pans_with_arrow_keys() {
    let fixture = CameraPanFixture::bootstrap();

    run_serial(&rspec::given(
        "PresentationPlugin camera responds to arrow key input",
        fixture,
        |scenario: &mut Scenario<CameraPanFixture>| {
            scenario.when("ArrowUp is pressed", |ctx| {
                ctx.before_each(|state| {
                    state.reset_state();
                    state.tick();
                    state.press_key(KeyCode::ArrowUp);
                    state.tick();
                });

                ctx.then("camera moves up", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    assert!(pos.y > 0.0, "ArrowUp should move camera up, got {pos:?}");
                });
            });

            scenario.when("ArrowDown is pressed", |ctx| {
                ctx.before_each(|state| {
                    state.reset_state();
                    state.tick();
                    state.press_key(KeyCode::ArrowDown);
                    state.tick();
                });

                ctx.then("camera moves down", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    assert!(
                        pos.y < 0.0,
                        "ArrowDown should move camera down, got {pos:?}"
                    );
                });
            });

            scenario.when("ArrowLeft is pressed", |ctx| {
                ctx.before_each(|state| {
                    state.reset_state();
                    state.tick();
                    state.press_key(KeyCode::ArrowLeft);
                    state.tick();
                });

                ctx.then("camera moves left", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    assert!(
                        pos.x < 0.0,
                        "ArrowLeft should move camera left, got {pos:?}"
                    );
                });
            });

            scenario.when("ArrowRight is pressed", |ctx| {
                ctx.before_each(|state| {
                    state.reset_state();
                    state.tick();
                    state.press_key(KeyCode::ArrowRight);
                    state.tick();
                });

                ctx.then("camera moves right", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    assert!(
                        pos.x > 0.0,
                        "ArrowRight should move camera right, got {pos:?}"
                    );
                });
            });
        },
    ));
}

#[test]
fn camera_diagonal_movement_is_normalised() {
    let fixture = CameraPanFixture::bootstrap();

    run_serial(&rspec::given(
        "diagonal movement does not exceed cardinal speed",
        fixture,
        |scenario: &mut Scenario<CameraPanFixture>| {
            scenario.when("W and D are pressed simultaneously", |ctx| {
                ctx.before_each(|state| {
                    state.reset_state();
                    state.tick();
                    state.press_key(KeyCode::KeyW);
                    state.press_key(KeyCode::KeyD);
                    state.tick();
                });

                ctx.then("camera moves diagonally at normalised speed", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    // Diagonal movement should be roughly sqrt(2)/2 in each axis,
                    // not 1.0 in each axis.
                    assert!(
                        pos.x > 0.0 && pos.y > 0.0,
                        "diagonal should move in both positive axes, got {pos:?}"
                    );
                    // With normalisation, X and Y components should be roughly equal.
                    let ratio = pos.x / pos.y;
                    assert!(
                        (ratio - 1.0).abs() < 0.1,
                        "diagonal components should be roughly equal, got ratio {ratio}"
                    );
                });
            });
        },
    ));
}
