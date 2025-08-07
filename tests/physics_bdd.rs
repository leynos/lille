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
use rstest::rstest;
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

    /// Generic assertion helper for components with tolerance checking.
    fn assert_component_values<T, F>(&self, name: &str, extract: F, expected: &[f32])
    where
        T: Component,
        F: Fn(&T) -> Vec<f32>,
    {
        let app = self.app.lock().expect("app lock");
        let entity = self.entity.expect("entity not spawned");
        let component = app
            .world
            .get::<T>(entity)
            .unwrap_or_else(|| panic!("missing {name}"));

        let actual = extract(component);
        let tolerance = 1e-3;

        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!(
                (a - e).abs() < tolerance,
                "Component {name} mismatch: expected {e}, got {a}",
            );
        }
    }

    /// Asserts the entity's transform equals the expected coordinates.
    fn assert_position(&self, x: f32, y: f32, z: f32) {
        self.assert_component_values::<Transform, _>(
            "Transform",
            |t| vec![t.translation.x, t.translation.y, t.translation.z],
            &[x, y, z],
        );
    }

    /// Asserts the entity's velocity matches the expected vector.
    fn assert_velocity(&self, vx: f32, vy: f32, vz: f32) {
        self.assert_component_values::<VelocityComp, _>(
            "VelocityComp",
            |v| vec![v.vx, v.vy, v.vz],
            &[vx, vy, vz],
        );
    }
}

#[rstest]
#[case::falling(
    "an unsupported entity",
    |world: &mut TestWorld| {
        world.spawn_block(Block { id: 1, x: 0, y: 0, z: -2 });
        world.spawn_entity(Transform::from_xyz(0.0, 0.0, 2.0), VelocityComp::default());
    },
    (0.0, 0.0, 1.0),
    (0.0, 0.0, GRAVITY_PULL as f32)
)]
#[case::standing_flat(
    "an entity on a flat block",
    |world: &mut TestWorld| {
        world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
        world.spawn_entity(Transform::from_xyz(0.0, 0.0, 1.0), VelocityComp::default());
    },
    (0.0, 0.0, 1.0),
    (0.0, 0.0, 0.0)
)]
#[case::standing_sloped(
    "an entity on a sloped block",
    |world: &mut TestWorld| {
        world.spawn_sloped_block(
            Block { id: 1, x: 0, y: 0, z: 0 },
            BlockSlope { block_id: 1, grad_x: 1.0.into(), grad_y: 0.0.into() },
        );
        world.spawn_entity(Transform::from_xyz(0.0, 0.0, 1.5), VelocityComp::default());
    },
    (0.0, 0.0, 1.5),
    (0.0, 0.0, 0.0)
)]
#[case::move_heights(
    "an entity moving across blocks of different heights",
    |world: &mut TestWorld| {
        world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
        world.spawn_block(Block { id: 2, x: 1, y: 0, z: 1 });
        world.spawn_entity(
            Transform::from_xyz(0.0, 0.0, 1.0),
            VelocityComp { vx: 1.0, vy: 0.0, vz: 0.0 },
        );
    },
    (1.0, 0.0, 2.0),
    (1.0, 0.0, 0.0)
)]
fn physics_scenarios(
    #[case] description: &'static str,
    #[case] setup: fn(&mut TestWorld),
    #[case] expected_pos: (f32, f32, f32),
    #[case] expected_vel: (f32, f32, f32),
) {
    rspec::run(&rspec::given(description, TestWorld::default(), |ctx| {
        ctx.before_each(setup);
        ctx.when("the simulation ticks once", |ctx| {
            ctx.before_each(|world| world.tick());
            ctx.then("the expected outcome occurs", move |world| {
                world.assert_position(expected_pos.0, expected_pos.1, expected_pos.2);
                world.assert_velocity(expected_vel.0, expected_vel.1, expected_vel.2);
            });
        });
    }));
}
