//! Behaviour-driven tests for physics and motion rules.
//!
//! These scenarios exercise the DBSP circuit via a headless Bevy app and use
//! `rust-rspec` to express expectations declaratively. The DBSP circuit is the
//! sole source of truth for inferred motion; Bevy merely applies its outputs.

use bevy::prelude::*;
use lille::{
    components::{Block, BlockSlope},
    DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL,
};
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct TestWorld {
    /// Shared Bevy app; `rspec` fixtures must implement `Clone + Send + Sync`.
    app: Arc<Mutex<App>>,
    entity: Option<Entity>,
}

impl fmt::Debug for TestWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestWorld")
            .field("entity", &self.entity)
            .finish()
    }
}

impl Default for TestWorld {
    fn default() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);
        Self {
            app: Arc::new(Mutex::new(app)),
            entity: None,
        }
    }
}

impl TestWorld {
    /// Spawns a block into the world.
    fn spawn_block(&mut self, block: Block) {
        let mut app = self.app.lock().expect("app lock");
        app.world.spawn(block);
    }

    /// Spawns a block together with its slope on the same entity.
    fn spawn_sloped_block(&mut self, block: Block, slope: BlockSlope) {
        let mut app = self.app.lock().expect("app lock");
        app.world.spawn((block, slope));
    }

    /// Spawns an entity at `transform` with the supplied velocity.
    fn spawn_entity(&mut self, transform: Transform, vel: VelocityComp) {
        let mut app = self.app.lock().expect("app lock");
        let id = app.world.spawn((DdlogId(1), transform, vel)).id();
        self.entity = Some(id);
    }

    /// Advances the simulation by one tick.
    fn tick(&mut self) {
        let mut app = self.app.lock().expect("app lock");
        app.update();
    }

    /// Asserts the entity's transform equals the expected coordinates.
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

    /// Asserts the entity's velocity matches the expected vector.
    fn assert_velocity(&self, vx: f32, vy: f32, vz: f32) {
        let app = self.app.lock().expect("app lock");
        let entity = self.entity.expect("entity not spawned");
        let vel = app
            .world
            .get::<VelocityComp>(entity)
            .expect("missing VelocityComp");
        let tolerance = 1e-3;
        assert!((vel.vx - vx).abs() < tolerance);
        assert!((vel.vy - vy).abs() < tolerance);
        assert!((vel.vz - vz).abs() < tolerance);
    }
}

#[test]
fn entity_falls_in_empty_space() {
    rspec::run(&rspec::given(
        "an unsupported entity",
        TestWorld::default(),
        |ctx| {
            ctx.before_each(|world| {
                world.spawn_block(Block {
                    id: 1,
                    x: 0,
                    y: 0,
                    z: -2,
                });
                world.spawn_entity(Transform::from_xyz(0.0, 0.0, 2.0), VelocityComp::default());
            });
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then("the entity descends under gravity", |world| {
                    world.assert_position(0.0, 0.0, 1.0);
                    world.assert_velocity(0.0, 0.0, GRAVITY_PULL as f32);
                });
            });
        },
    ));
}

#[test]
fn entity_stands_on_flat_block() {
    rspec::run(&rspec::given(
        "an entity on a flat block",
        TestWorld::default(),
        |ctx| {
            ctx.before_each(|world| {
                world.spawn_block(Block {
                    id: 1,
                    x: 0,
                    y: 0,
                    z: 0,
                });
                world.spawn_entity(Transform::from_xyz(0.0, 0.0, 1.0), VelocityComp::default());
            });
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then("the entity remains on the block", |world| {
                    world.assert_position(0.0, 0.0, 1.0);
                    world.assert_velocity(0.0, 0.0, 0.0);
                });
            });
        },
    ));
}

#[test]
fn entity_stands_on_sloped_block() {
    rspec::run(&rspec::given(
        "an entity on a sloped block",
        TestWorld::default(),
        |ctx| {
            ctx.before_each(|world| {
                world.spawn_sloped_block(
                    Block {
                        id: 1,
                        x: 0,
                        y: 0,
                        z: 0,
                    },
                    BlockSlope {
                        block_id: 1,
                        grad_x: 1.0.into(),
                        grad_y: 0.0.into(),
                    },
                );
                world.spawn_entity(Transform::from_xyz(0.0, 0.0, 1.5), VelocityComp::default());
            });
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then("the entity stays aligned with the slope", |world| {
                    world.assert_position(0.0, 0.0, 1.5);
                    world.assert_velocity(0.0, 0.0, 0.0);
                });
            });
        },
    ));
}

#[test]
fn entity_moves_between_heights() {
    rspec::run(&rspec::given(
        "an entity moving across blocks of different heights",
        TestWorld::default(),
        |ctx| {
            ctx.before_each(|world| {
                world.spawn_block(Block {
                    id: 1,
                    x: 0,
                    y: 0,
                    z: 0,
                });
                world.spawn_block(Block {
                    id: 2,
                    x: 1,
                    y: 0,
                    z: 1,
                });
                world.spawn_entity(
                    Transform::from_xyz(0.0, 0.0, 1.0),
                    VelocityComp {
                        vx: 1.0,
                        vy: 0.0,
                        vz: 0.0,
                    },
                );
            });
            ctx.when("the simulation ticks once", |ctx| {
                ctx.before_each(|world| world.tick());
                ctx.then("the entity snaps to the new floor height", |world| {
                    world.assert_position(1.0, 0.0, 2.0);
                    world.assert_velocity(1.0, 0.0, 0.0);
                });
            });
        },
    ));
}
