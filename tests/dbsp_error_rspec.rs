//! Behavioural test covering DBSP error observability under the new Events V2
//! pipeline.

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

use bevy::prelude::*;
use lille::dbsp_sync::DbspState;
use lille::{DbspCircuit, DbspPlugin, DbspSyncErrorContext, DdlogId, Health, VelocityComp};
use log::error;
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::{lock_app, SharedApp, ThreadSafeApp};

#[path = "support/dbsp_error_capture.rs"]
mod dbsp_test_support;

fn failing_step(_: &mut DbspCircuit) -> Result<(), dbsp::Error> {
    Err(dbsp::Error::IO(io::Error::other("forced failure")))
}

#[derive(Debug, Clone)]
struct DbspErrorFixture {
    app: SharedApp,
    entity: Entity,
}

impl DbspErrorFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        dbsp_test_support::install_error_observer(&mut app);
        app.add_plugins(DbspPlugin);
        let entity = app
            .world_mut()
            .spawn((
                DdlogId(1),
                Transform::from_translation(Vec3::new(7.0, 0.0, 0.0)),
                VelocityComp::default(),
                Health {
                    current: 90,
                    max: 100,
                },
            ))
            .id();
        Self {
            app: Arc::new(Mutex::new(ThreadSafeApp(app))),
            entity,
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        lock_app(&self.app)
    }

    fn reset(&self) {
        let mut app = self.app_guard();
        app.world_mut()
            .non_send_resource_mut::<DbspState>()
            .set_stepper_for_testing(failing_step);
        app.world_mut()
            .resource_mut::<dbsp_test_support::CapturedErrors>()
            .0
            .clear();
        if let Some(mut transform) = app.world_mut().get_mut::<Transform>(self.entity) {
            transform.translation = Vec3::new(7.0, 0.0, 0.0);
        }
    }

    fn tick(&self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.app_guard().update();
        }));
        if let Err(payload) = result {
            error!(
                "tick panicked: {}",
                payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("non-string panic payload")
            );
            std::panic::resume_unwind(payload);
        }
    }

    fn errors(&self) -> Vec<(String, String)> {
        self.app_guard()
            .world()
            .resource::<dbsp_test_support::CapturedErrors>()
            .0
            .clone()
    }

    fn entity_position(&self) -> Vec3 {
        self.app_guard()
            .world()
            .get::<Transform>(self.entity)
            .map_or(Vec3::ZERO, |t| t.translation)
    }
}

#[test]
fn dbsp_step_failure_is_observed_and_non_destructive() {
    let fixture = DbspErrorFixture::bootstrap();
    run_serial(&rspec::given(
        "a DBSP step failure is triggered",
        fixture,
        |scenario: &mut Scenario<DbspErrorFixture>| {
            scenario.before_each(|state| {
                state.reset();
                state.tick();
            });

            scenario.then("the observer records a DbspSyncError", |state| {
                let errors = state.errors();
                let error = errors
                    .first()
                    .expect("error event should be recorded via observer");
                assert_eq!(errors.len(), 1);
                assert_eq!(error.0, format!("{:?}", DbspSyncErrorContext::Step));
                assert!(error.1.contains("forced failure"));
            });

            scenario.then("existing ECS data remains unchanged", |state| {
                assert_eq!(state.entity_position(), Vec3::new(7.0, 0.0, 0.0));
            });
        },
    ));
}
