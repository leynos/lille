#![cfg_attr(
    feature = "render",
    doc = "Behavioural tests ensuring spawned entities flow through the DBSP cache."
)]
#![cfg_attr(
    not(feature = "render"),
    doc = "Behavioural tests require the `render` feature."
)]
#![cfg(feature = "render")]
//! Validates that demo entities spawned with required components are mirrored
//! through the DBSP cache, keeping the circuit authoritative for inferred
//! behaviour even when component data goes missing.

#[path = "support/thread_safe_app.rs"]
mod thread_safe_app;

#[path = "support/rspec_runner.rs"]
mod rspec_runner;

use std::sync::{Arc, Mutex, MutexGuard};

use bevy::prelude::*;
use lille::{DbspPlugin, DdlogId, WorldHandle};
use rspec::block::Context as Scenario;
use rspec_runner::run_serial;
use thread_safe_app::{lock_app, SharedApp, ThreadSafeApp};

use lille::spawn_world_system;

#[derive(Debug, Clone)]
struct SpawnDbspFixture {
    app: SharedApp,
}

impl SpawnDbspFixture {
    fn bootstrap() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(DbspPlugin);
        app.add_systems(Startup, spawn_world_system);
        Self {
            app: Arc::new(Mutex::new(ThreadSafeApp(app))),
        }
    }

    fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        lock_app(&self.app)
    }

    fn tick(&self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.app_guard().update();
        }));
        if let Err(payload) = result {
            bevy::log::error!(
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

    fn cached_ids(&self) -> Vec<i64> {
        let app = self.app_guard();
        let handle = app
            .world()
            .get_resource::<WorldHandle>()
            .unwrap_or_else(|| panic!("WorldHandle should exist after DBSP startup"));
        let mut ids: Vec<i64> = handle.entity_ids().collect();
        ids.sort_unstable();
        ids
    }

    fn remove_transform(&self, ddlog_id: i64) -> bool {
        let mut app = self.app_guard();
        let mut query = app.world_mut().query::<(Entity, &DdlogId)>();
        let Some((entity, _)) = query.iter(app.world()).find(|(_, id)| id.0 == ddlog_id) else {
            return false;
        };
        app.world_mut().entity_mut(entity).remove::<Transform>();
        true
    }
}

#[test]
fn dbsp_caches_spawned_entities() {
    let fixture = SpawnDbspFixture::bootstrap();
    run_serial(&rspec::given(
        "spawn_world_system seeds demo entities",
        fixture,
        |scenario: &mut Scenario<SpawnDbspFixture>| {
            scenario.before_each(|state| state.tick());

            scenario.then(
                "DBSP caches all spawned DdlogIds from the ECS world",
                |state| {
                    let ids = state.cached_ids();
                    assert_eq!(ids, vec![1, 2, 3], "cached ids after spawn: {ids:?}");
                },
            );

            scenario.then(
                "DBSP drops entities without transforms on subsequent ticks",
                |state| {
                    assert!(
                        state.remove_transform(2),
                        "Civvy entity with DdlogId 2 should exist"
                    );
                    state.tick();
                    let ids = state.cached_ids();
                    assert_eq!(ids, vec![1, 3], "cached ids after transform drop: {ids:?}");
                },
            );
        },
    ));
}

#[test]
fn cached_ids_drop_when_transform_missing() {
    let fixture = SpawnDbspFixture::bootstrap();
    fixture.tick();
    let mut ids = fixture.cached_ids();
    assert_eq!(ids, vec![1, 2, 3]);
    assert!(fixture.remove_transform(2));
    fixture.tick();
    ids = fixture.cached_ids();
    assert_eq!(ids, vec![1, 3]);
}
