#![cfg_attr(
    feature = "test-support",
    doc = "Behavioural tests for `PresentationPlugin` using rust-rspec."
)]
#![cfg_attr(
    not(feature = "test-support"),
    doc = "Behavioural tests require `test-support`."
)]
#![cfg(feature = "test-support")]
//! Behavioural test: `PresentationPlugin` spawns a camera with `CameraController` marker.
//!
//! This file contains a single test because it ticks the Bevy app under
//! `--all-features`, which initialises a render device and uses process-global
//! renderer state.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

#[path = "support/map_fixture.rs"]
mod map_fixture;

use std::sync::MutexGuard;

use bevy::prelude::*;
use lille::presentation::CameraController;
use lille::PresentationPlugin;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::ThreadSafeApp;

/// Fixture for presentation plugin behavioural tests.
#[derive(Debug, Clone)]
struct PresentationPluginFixture {
    base: map_fixture::MapPluginFixtureBase,
}

impl PresentationPluginFixture {
    /// Creates a test fixture with the `PresentationPlugin` installed.
    fn bootstrap() -> Self {
        let mut app = App::new();
        map_test_plugins::add_map_test_plugins(&mut app);
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

    /// Returns the count of entities with `CameraController` component.
    fn camera_controller_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&CameraController>();
        query.iter(world).count()
    }

    /// Returns the count of entities with `Camera2d` component.
    fn camera_2d_count(&self) -> usize {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<&Camera2d>();
        query.iter(world).count()
    }

    /// Returns true if the camera entity has both `CameraController` and `Camera2d`.
    fn camera_is_properly_configured(&self) -> bool {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&CameraController, &Camera2d)>();
        query.iter(world).next().is_some()
    }

    /// Returns the name of the camera entity if it has one.
    fn camera_name(&self) -> Option<String> {
        let mut app = self.app_guard();
        let world = app.world_mut();
        let mut query = world.query::<(&CameraController, &Name)>();
        query
            .iter(world)
            .next()
            .map(|(_, name)| name.as_str().to_owned())
    }
}

#[test]
fn presentation_plugin_spawns_camera_with_controller_marker() {
    let fixture = PresentationPluginFixture::bootstrap();

    run_serial(&rspec::given(
        "PresentationPlugin initialises camera setup",
        fixture,
        |scenario: &mut Scenario<PresentationPluginFixture>| {
            scenario.when("the app ticks once", |ctx| {
                ctx.before_each(|state| {
                    state.tick();
                });

                ctx.then("exactly one Camera2d entity exists", |state| {
                    assert_eq!(
                        state.camera_2d_count(),
                        1,
                        "expected exactly one Camera2d entity"
                    );
                });

                ctx.then("exactly one CameraController entity exists", |state| {
                    assert_eq!(
                        state.camera_controller_count(),
                        1,
                        "expected exactly one CameraController entity"
                    );
                });

                ctx.then(
                    "the camera has both Camera2d and CameraController",
                    |state| {
                        assert!(
                            state.camera_is_properly_configured(),
                            "expected camera to have both Camera2d and CameraController"
                        );
                    },
                );

                ctx.then("the camera is named PresentationCamera", |state| {
                    assert_eq!(
                        state.camera_name(),
                        Some("PresentationCamera".to_owned()),
                        "expected camera to be named PresentationCamera"
                    );
                });
            });
        },
    ));
}
