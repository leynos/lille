//! Shared helpers for physics BDD tests.

use approx::assert_relative_eq;
use bevy::prelude::*;
use lille::numeric::expect_f32;
use lille::{
    components::{Block, BlockSlope, ForceComp},
    DbspPlugin, DdlogId, Health, VelocityComp,
};
use rstest::fixture;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Wraps Bevy's `App` to provide explicit `Send` and `Sync` guarantees for
/// tests that serialise access through a mutex.
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

// SAFETY: Tests execute on a single thread via rspec and all access is guarded
// by the mutex, so forwarding `Send`/`Sync` to the wrapper is sound here.
unsafe impl Send for ThreadSafeApp {}
unsafe impl Sync for ThreadSafeApp {}

/// Test harness wrapping a Bevy app configured for DBSP scenarios.
#[derive(Clone)]
pub struct TestWorld {
    /// Shared Bevy app; `rspec` fixtures must implement `Clone + Send + Sync`.
    app: Arc<Mutex<ThreadSafeApp>>,
    pub entity: Option<Entity>,
    expected_damage: Arc<Mutex<Option<u16>>>,
    initial_health: Arc<Mutex<Option<u16>>>,
    /// Monotonic generator for unique DDlog identifiers used in tests.
    next_ddlog_id: Arc<Mutex<i64>>,
}

impl fmt::Debug for TestWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestWorld")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

impl Default for TestWorld {
    fn default() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);
        Self {
            app: Arc::new(Mutex::new(ThreadSafeApp(app))),
            entity: None,
            expected_damage: Arc::new(Mutex::new(None)),
            initial_health: Arc::new(Mutex::new(None)),
            next_ddlog_id: Arc::new(Mutex::new(1)),
        }
    }
}

impl TestWorld {
    /// Acquire the underlying Bevy `App`, recovering the guard if the mutex is poisoned.
    pub fn app_guard(&self) -> MutexGuard<'_, ThreadSafeApp> {
        self.app.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Access the expected-damage slot, recovering the guard if the mutex is poisoned.
    pub fn expected_damage_guard(&self) -> MutexGuard<'_, Option<u16>> {
        self.expected_damage
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Access the stored initial health, panicking if the mutex is poisoned.
    pub fn initial_health_guard(&self) -> MutexGuard<'_, Option<u16>> {
        self.initial_health
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Record the entity's initial health for later assertions.
    pub fn set_initial_health(&self, health: u16) {
        *self.initial_health_guard() = Some(health);
    }

    /// Return the spawned entity or panic if not initialised.
    pub fn entity_or_panic(&self) -> Entity {
        self.entity.unwrap_or_else(|| panic!("entity not spawned"))
    }

    /// Spawns a block into the world.
    pub fn spawn_block(&mut self, block: Block) {
        self.app_guard().world_mut().spawn(block);
    }

    /// Spawns a block together with its slope on the same entity.
    pub fn spawn_sloped_block(&mut self, block: Block, slope: BlockSlope) {
        self.app_guard().world_mut().spawn((block, slope));
    }

    /// Spawns an entity at `transform` with the supplied velocity.
    pub fn spawn_entity(
        &mut self,
        transform: Transform,
        vel: VelocityComp,
        force: Option<ForceComp>,
    ) {
        let entity_id = {
            let mut app = self.app_guard();
            let mut id_guard = self
                .next_ddlog_id
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let ddlog_id = *id_guard;
            *id_guard += 1;
            let mut entity = app.world_mut().spawn((DdlogId(ddlog_id), transform, vel));
            if let Some(force_comp) = force {
                entity.insert(force_comp);
            }
            entity.id()
        };
        self.entity = Some(entity_id);
    }

    /// Spawns an entity without an external force.
    pub fn spawn_entity_without_force(&mut self, transform: Transform, vel: VelocityComp) {
        self.spawn_entity(transform, vel, None);
    }

    /// Spawns an entity with an attached health component.
    pub fn spawn_entity_with_health(
        &mut self,
        transform: Transform,
        vel: VelocityComp,
        health: Health,
    ) {
        let entity_id = {
            let mut app = self.app_guard();
            let mut id_guard = self
                .next_ddlog_id
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let ddlog_id = *id_guard;
            *id_guard += 1;
            let entity = app
                .world_mut()
                .spawn((DdlogId(ddlog_id), transform, vel, health));
            entity.id()
        };
        self.entity = Some(entity_id);
        self.set_initial_health(health.current);
    }

    /// Spawns an entity without registering it with the DBSP circuit.
    pub fn spawn_orphan_entity(&mut self, transform: Transform, vel: VelocityComp) {
        let entity_id = {
            let mut app = self.app_guard();
            app.world_mut().spawn((transform, vel)).id()
        };
        self.entity = Some(entity_id);
    }

    /// Despawns the currently tracked entity if it exists.
    pub fn despawn_tracked_entity(&mut self) {
        if let Some(entity) = self.entity.take() {
            let mut app = self.app_guard();
            if app.world().get_entity(entity).is_some() {
                app.world_mut().entity_mut(entity).despawn_recursive();
            }
        }
    }

    /// Fetch the entity's `Health` component, panicking if it is missing.
    pub fn health(&self) -> Health {
        let app = self.app_guard();
        let entity = self.entity_or_panic();
        app.world()
            .get::<Health>(entity)
            .cloned()
            .unwrap_or_else(|| panic!("missing Health component"))
    }

    /// Mutate the entity's `Transform` Z translation.
    pub fn set_position_z(&self, z: f32) {
        let mut app = self.app_guard();
        let entity = self.entity_or_panic();
        let Some(mut transform) = app.world_mut().get_mut::<Transform>(entity) else {
            panic!("missing Transform component");
        };
        transform.translation.z = z;
    }

    /// Mutate the entity's `VelocityComp` vertical component.
    pub fn set_velocity_z(&self, vz: f32) {
        let mut app = self.app_guard();
        let entity = self.entity_or_panic();
        let Some(mut velocity) = app.world_mut().get_mut::<VelocityComp>(entity) else {
            panic!("missing VelocityComp component");
        };
        velocity.vz = vz;
    }

    /// Record an expected damage value for subsequent assertions.
    pub fn set_expected_damage(&self, damage: u16) {
        *self.expected_damage_guard() = Some(damage);
    }

    /// Retrieve and clear the expected damage value, panicking if unset.
    pub fn take_expected_damage(&self) -> u16 {
        let mut expected = self.expected_damage_guard();
        expected
            .take()
            .unwrap_or_else(|| panic!("expected damage should be recorded"))
    }

    /// Retrieve and clear the initial health, panicking if unset.
    pub fn take_initial_health(&self) -> u16 {
        let mut initial = self.initial_health_guard();
        initial
            .take()
            .unwrap_or_else(|| panic!("initial health should be recorded"))
    }

    /// Advances the simulation by one tick.
    pub fn tick(&mut self) {
        self.app_guard().update();
    }

    /// Generic assertion helper for components with tolerance checking.
    pub fn assert_component_values<T, F>(&self, name: &str, extract: F, expected: &[f32])
    where
        T: Component,
        F: Fn(&T) -> Vec<f32>,
    {
        let app = self.app_guard();
        let entity = self.entity_or_panic();
        let Some(component) = app.world().get::<T>(entity) else {
            panic!("missing {name}");
        };

        let actual = extract(component);
        let tolerance = 1e-3;

        assert_eq!(
            actual.len(),
            expected.len(),
            "mismatched component arity for {name}: actual={}, expected={}",
            actual.len(),
            expected.len()
        );

        for (a, e) in actual.iter().zip(expected.iter()) {
            assert_relative_eq!(*a, *e, epsilon = tolerance);
        }
    }

    /// Asserts the entity's transform equals the expected coordinates.
    pub fn assert_position(&self, x: f32, y: f32, z: f32) {
        self.assert_component_values::<Transform, _>(
            "Transform",
            |t| vec![t.translation.x, t.translation.y, t.translation.z],
            &[x, y, z],
        );
    }

    /// Asserts the entity's velocity matches the expected vector.
    pub fn assert_velocity(&self, vx: f32, vy: f32, vz: f32) {
        self.assert_component_values::<VelocityComp, _>(
            "VelocityComp",
            |v| vec![v.vx, v.vy, v.vz],
            &[vx, vy, vz],
        );
    }
}

/// Provides a fresh Bevy world for each scenario.
/// Provide a fresh `TestWorld` for rstest fixtures.
#[fixture]
pub fn world() -> TestWorld {
    TestWorld::default()
}

/// Shortcut type for setup functions used by scenarios.
pub type SetupFn = fn(&mut TestWorld);

/// Describes a physics scenario with expected position/velocity.
#[derive(Clone, Copy)]
pub struct PhysicsScenario {
    pub setup: SetupFn,
    pub expected_position: (f32, f32, f32),
    pub expected_velocity: (f32, f32, f32),
}

/// Build a `PhysicsScenario` from double-precision expectations.
pub fn physics_scenario(
    setup: SetupFn,
    expected_position: (f64, f64, f64),
    expected_velocity: (f64, f64, f64),
) -> PhysicsScenario {
    PhysicsScenario {
        setup,
        expected_position: (
            expect_f32(expected_position.0),
            expect_f32(expected_position.1),
            expect_f32(expected_position.2),
        ),
        expected_velocity: (
            expect_f32(expected_velocity.0),
            expect_f32(expected_velocity.1),
            expect_f32(expected_velocity.2),
        ),
    }
}

/// Execute the scenario against a `TestWorld` and assert outputs.
pub fn run_physics_scenario(mut world: TestWorld, scenario: PhysicsScenario) {
    (scenario.setup)(&mut world);
    world.tick();
    let (px, py, pz) = scenario.expected_position;
    world.assert_position(px, py, pz);
    let (vx, vy, vz) = scenario.expected_velocity;
    world.assert_velocity(vx, vy, vz);
}

/// Assert the recorded expected damage matches `expected`.
pub fn assert_expected_damage(world: &TestWorld, expected: u16) {
    assert_eq!(world.take_expected_damage(), expected);
}

/// Spawn multiple blocks specified as `(x, y, z)` triples.
pub fn spawn_blocks(world: &mut TestWorld, blocks: &[(i32, i32, i32)]) {
    for (idx, &(x, y, z)) in blocks.iter().enumerate() {
        world.spawn_block(Block {
            id: idx as i64,
            x,
            y,
            z,
        });
    }
}
