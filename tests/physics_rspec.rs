//! Behaviour-driven tests using rust-rspec.
//!
//! These tests verify physics behaviour in a headless Bevy application.
//! The tests use the rspec framework to provide BDD-style test structure for
//! physics simulation scenarios, particularly gravity effects on entities.

use bevy::prelude::*;
use lille::{components::Block, DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL};
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct PhysicsWorld {
    app: Arc<Mutex<App>>,
    entity: Option<Entity>,
}

impl fmt::Debug for PhysicsWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PhysicsWorld")
            .field("entity", &self.entity)
            .finish()
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            app: Arc::new(Mutex::new(App::new())),
            entity: None,
        }
    }
}

impl PhysicsWorld {
    fn setup(&mut self) {
        if self.entity.is_some() {
            return; // Already set up
        }
        let mut app = self.app.lock().expect("app lock");
        app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);
        app.world.spawn(Block {
            id: 1,
            x: 0,
            y: 0,
            z: -10,
        });
        let id = app
            .world
            .spawn((
                DdlogId(1),
                Transform::from_xyz(0.0, 0.0, 2.0),
                VelocityComp::default(),
            ))
            .id();
        self.entity = Some(id);
    }

    fn tick(&mut self) {
        let mut app = self.app.lock().expect("app lock");
        app.update();
    }

    /// Asserts the entity's z-position matches `expected_z` and its vertical
    /// velocity equals the expected gravity-induced value.
    fn assert_z_and_velocity(&self, expected_z: f32) {
        let app = self.app.lock().expect("app lock");
        let entity = self.entity.expect("entity not spawned");
        let transform = app
            .world
            .get::<Transform>(entity)
            .expect("entity should have Transform component");
        let actual_z = transform.translation.z;
        let tolerance = 1e-3;
        assert!(
            (actual_z - expected_z).abs() < tolerance,
            "expected z {expected_z}, got {actual_z}"
        );
        let vel = app
            .world
            .get::<VelocityComp>(entity)
            .expect("entity should have VelocityComp component");
        // VelocityComp::default() yields zero initial velocity. The expected
        // value after one tick equals that baseline plus the gravity pull.
        let expected_vz = VelocityComp::default().vz + GRAVITY_PULL as f32;
        assert!(
            (vel.vz - expected_vz).abs() < tolerance,
            "expected vz {} got {}",
            expected_vz,
            vel.vz
        );
    }
}

#[test]
fn unsupported_entity_falls() {
    rspec::run(&rspec::given(
        "a headless app with a single unsupported entity",
        PhysicsWorld::default(),
        |ctx| {
            ctx.before_each(|world| world.setup());
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then("the entity's z position should be 1.0", |world| {
                    world.assert_z_and_velocity(1.0);
                });
            });
        },
    ));
}
