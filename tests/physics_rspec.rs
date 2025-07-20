//! Behaviour-driven tests using rust-rspec.
//!
//! These tests verify that unsupported entities fall under gravity in a
//! headless Bevy application.

use bevy::prelude::*;
use lille::{DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL};
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
        let mut app = self.app.lock().expect("app lock");
        app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);
        let id = app
            .world
            .spawn((
                DdlogId(1),
                Transform::from_xyz(0.0, 0.0, 1.0),
                VelocityComp::default(),
            ))
            .id();
        self.entity = Some(id);
    }

    fn tick(&mut self) {
        let mut app = self.app.lock().expect("app lock");
        app.update();
    }

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
        assert!(
            (vel.vz - GRAVITY_PULL as f32).abs() < tolerance,
            "expected vz {} got {}",
            GRAVITY_PULL as f32,
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
                ctx.then("the entity's z position should be 0.0", |world| {
                    world.assert_z_and_velocity(0.0);
                });
            });
        },
    ));
}
