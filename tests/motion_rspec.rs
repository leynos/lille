//! Behaviour tests for entity motion using rust-rspec.
//!
//! Verifies that standing entities move across blocks and snap to the new
//! floor height.

use bevy::prelude::*;
use lille::{components::Block, DbspPlugin, DdlogId, VelocityComp};
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct GroundWorld {
    /// Shared Bevy app; `rspec` requires fixtures to be `Clone + Send + Sync`.
    app: Arc<Mutex<App>>,
    entity: Option<Entity>,
}

impl fmt::Debug for GroundWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GroundWorld")
            .field("entity", &self.entity)
            .finish()
    }
}

impl Default for GroundWorld {
    fn default() -> Self {
        Self {
            app: Arc::new(Mutex::new(App::new())),
            entity: None,
        }
    }
}

impl GroundWorld {
    fn setup(&mut self) {
        if self.entity.is_some() {
            return;
        }
        let mut app = self.app.lock().expect("app lock");
        app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);
        app.world.spawn(Block {
            id: 1,
            x: 0,
            y: 0,
            z: 0,
        });
        app.world.spawn(Block {
            id: 2,
            x: 1,
            y: 0,
            z: 1,
        });
        let id = app
            .world
            .spawn((
                DdlogId(1),
                Transform::from_xyz(0.0, 0.0, 1.0),
                VelocityComp {
                    vx: 1.0,
                    vy: 0.0,
                    vz: 0.0,
                },
            ))
            .id();
        self.entity = Some(id);
    }

    fn tick(&mut self) {
        let mut app = self.app.lock().expect("app lock");
        app.update();
    }

    fn assert_position(&self, x: f32, y: f32, z: f32) {
        let app = self.app.lock().expect("app lock");
        let entity = self.entity.expect("entity not spawned");
        let transform = app
            .world
            .get::<Transform>(entity)
            .expect("missing Transform");
        let tolerance = 1e-3;
        assert!((transform.translation.x - x).abs() < tolerance);
        assert!((transform.translation.y - y).abs() < tolerance);
        assert!((transform.translation.z - z).abs() < tolerance);
    }

    fn with_velocity_component<F, R>(&self, mut f: F) -> R
    where
        F: FnMut(&mut VelocityComp) -> R,
    {
        let mut app = self.app.lock().expect("app lock");
        let entity = self.entity.expect("entity not spawned");
        let mut vel = app
            .world
            .get_mut::<VelocityComp>(entity)
            .expect("missing VelocityComp");
        f(&mut vel)
    }

    fn set_entity_vertical_velocity(&mut self, vz: f32) {
        self.with_velocity_component(|vel| {
            vel.vz = vz;
        });
    }

    fn assert_entity_vertical_velocity(&self, expected: f32) {
        self.with_velocity_component(|vel| {
            let tolerance = 1e-3;
            assert!((vel.vz - expected).abs() < tolerance);
        });
    }
}

#[test]
fn standing_entity_moves_and_snaps() {
    rspec::run(&rspec::given(
        "a world with two blocks and a standing entity",
        GroundWorld::default(),
        |ctx| {
            ctx.before_each(|world| world.setup());
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then(
                    "the entity moves to the second block and snaps to its height",
                    |world| {
                        world.assert_position(1.0, 0.0, 2.0);
                    },
                );
            });
        },
    ));
}

#[test]
fn standing_entity_with_vertical_velocity_resets_velocity() {
    rspec::run(&rspec::given(
        "a world with a standing entity with vertical velocity",
        GroundWorld::default(),
        |ctx| {
            ctx.before_each(|world| {
                world.setup();
                world.set_entity_vertical_velocity(5.0);
            });
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then("the entity's vertical velocity is reset to zero", |world| {
                    world.assert_entity_vertical_velocity(0.0);
                });
            });
        },
    ));
}
