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
//! expected direction and that diagonal movement is normalized.

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
            max_delta_seconds: 1.0, // Deterministic timing for tests
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
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.reset_all();
        }
        // Reset camera X/Y to origin, preserving Z (camera depth).
        let world = app.world_mut();
        let mut query = world.query::<(&mut Transform, &CameraController)>();
        for (mut transform, _) in query.iter_mut(world) {
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
        }
    }

    /// Returns the camera position, enforcing exactly one `CameraController` exists.
    ///
    /// # Panics
    ///
    /// Panics if zero or more than one camera entity exists.
    fn camera_position(&self) -> Option<Vec3> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&Transform, &CameraController)>();
        let mut cameras = query.iter(world);
        let first = cameras.next();
        let second = cameras.next();
        match (first, second) {
            (None, _) => None,
            (Some((transform, _)), None) => Some(transform.translation),
            _ => panic!("expected exactly one CameraController, found multiple"),
        }
    }
}

/// Scenario descriptions for key movement tests.
#[derive(Clone, Copy)]
struct KeyMovementDesc {
    when_desc: &'static str,
    then_desc: &'static str,
    error_msg: &'static str,
}

/// Helper to test that pressing a key moves the camera in the expected direction.
fn test_key_movement<F>(
    scenario: &mut Scenario<CameraPanFixture>,
    key: KeyCode,
    desc: KeyMovementDesc,
    assertion: F,
) where
    F: Fn(&Vec3) -> bool + Clone + 'static,
{
    scenario.when(desc.when_desc, move |ctx| {
        ctx.before_each(move |state| {
            state.tick(); // Spawn camera first
            state.reset_state();
            state.tick();
            state.press_key(key);
            state.tick();
        });

        let check = assertion.clone();
        let error_msg = desc.error_msg;
        ctx.then(desc.then_desc, move |state| {
            let pos = state
                .camera_position()
                .unwrap_or_else(|| panic!("camera should exist"));
            assert!(check(&pos), "{error_msg}, got {pos:?}");
        });
    });
}

/// Helper to test that no movement occurs when no keys are pressed.
fn test_no_movement(scenario: &mut Scenario<CameraPanFixture>) {
    scenario.when("no movement keys are pressed", |ctx| {
        ctx.before_each(|state| {
            state.tick(); // Spawn camera first
            state.reset_state();
            state.tick();
            state.tick();
        });

        ctx.then("camera position remains unchanged", |state| {
            let pos = state
                .camera_position()
                .unwrap_or_else(|| panic!("camera should exist"));
            assert!(
                pos.x.abs() < 0.001 && pos.y.abs() < 0.001,
                "camera should not move when no keys pressed, got {pos:?}"
            );
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
            scenario.when("the app initializes", |ctx| {
                ctx.before_each(|state| {
                    state.tick(); // Finalize plugins and spawn camera
                    state.reset_state(); // Ensure clean state for test
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
                KeyMovementDesc {
                    when_desc: "W key is pressed",
                    then_desc: "camera moves up (positive Y)",
                    error_msg: "camera Y should increase when W pressed",
                },
                |pos| pos.y > 0.0,
            );
            test_key_movement(
                scenario,
                KeyCode::KeyS,
                KeyMovementDesc {
                    when_desc: "S key is pressed",
                    then_desc: "camera moves down (negative Y)",
                    error_msg: "camera Y should decrease when S pressed",
                },
                |pos| pos.y < 0.0,
            );
            test_key_movement(
                scenario,
                KeyCode::KeyA,
                KeyMovementDesc {
                    when_desc: "A key is pressed",
                    then_desc: "camera moves left (negative X)",
                    error_msg: "camera X should decrease when A pressed",
                },
                |pos| pos.x < 0.0,
            );
            test_key_movement(
                scenario,
                KeyCode::KeyD,
                KeyMovementDesc {
                    when_desc: "D key is pressed",
                    then_desc: "camera moves right (positive X)",
                    error_msg: "camera X should increase when D pressed",
                },
                |pos| pos.x > 0.0,
            );

            test_no_movement(scenario);
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
            test_key_movement(
                scenario,
                KeyCode::ArrowUp,
                KeyMovementDesc {
                    when_desc: "ArrowUp is pressed",
                    then_desc: "camera moves up",
                    error_msg: "ArrowUp should move camera up",
                },
                |pos| pos.y > 0.0,
            );
            test_key_movement(
                scenario,
                KeyCode::ArrowDown,
                KeyMovementDesc {
                    when_desc: "ArrowDown is pressed",
                    then_desc: "camera moves down",
                    error_msg: "ArrowDown should move camera down",
                },
                |pos| pos.y < 0.0,
            );
            test_key_movement(
                scenario,
                KeyCode::ArrowLeft,
                KeyMovementDesc {
                    when_desc: "ArrowLeft is pressed",
                    then_desc: "camera moves left",
                    error_msg: "ArrowLeft should move camera left",
                },
                |pos| pos.x < 0.0,
            );
            test_key_movement(
                scenario,
                KeyCode::ArrowRight,
                KeyMovementDesc {
                    when_desc: "ArrowRight is pressed",
                    then_desc: "camera moves right",
                    error_msg: "ArrowRight should move camera right",
                },
                |pos| pos.x > 0.0,
            );
        },
    ));
}

/// Minimum epsilon based on tick size to prevent "barely moved" passes.
const MIN_MOVEMENT: f32 = 0.01;

#[test]
fn camera_diagonal_movement_is_normalized() {
    let fixture = CameraPanFixture::bootstrap();

    run_serial(&rspec::given(
        "diagonal movement does not exceed cardinal speed",
        fixture,
        |scenario: &mut Scenario<CameraPanFixture>| {
            scenario.when("W and D are pressed simultaneously", |ctx| {
                ctx.before_each(|state| {
                    state.tick(); // Spawn camera first
                    state.reset_state();
                    state.tick();
                    state.press_key(KeyCode::KeyW);
                    state.press_key(KeyCode::KeyD);
                    state.tick();
                });

                ctx.then("camera moves diagonally at normalized speed", |state| {
                    let pos = state.camera_position().expect("camera should exist");
                    // Diagonal movement should be roughly sqrt(2)/2 in each axis,
                    // not 1.0 in each axis.
                    assert!(
                        pos.x > MIN_MOVEMENT && pos.y > MIN_MOVEMENT,
                        "diagonal should move significantly in both axes, got {pos:?}"
                    );
                    // With normalization, X and Y components should be roughly equal.
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
