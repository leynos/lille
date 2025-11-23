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

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use bevy::prelude::*;
use lille::{DbspPlugin, DdlogId, WorldHandle};
use rspec::block::Context as Scenario;
use std::ops::{Deref, DerefMut};

use lille::spawn_world_system;

#[derive(Debug)]
struct ThreadSafeApp(App);

impl Deref for ThreadSafeApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ThreadSafeApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// SAFETY: Access to the wrapped `App` is serialised through the mutex in tests.
unsafe impl Send for ThreadSafeApp {}
unsafe impl Sync for ThreadSafeApp {}

#[derive(Debug)]
struct SpawnDbspFixture {
    app: Arc<Mutex<ThreadSafeApp>>,
}

impl Clone for SpawnDbspFixture {
    fn clone(&self) -> Self {
        Self::bootstrap()
    }
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
        self.app.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn tick(&self) {
        self.app_guard().update();
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
    rspec::run(&rspec::given(
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
