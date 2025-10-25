//! Behaviour-driven tests for physics and motion rules.
//!
//! These scenarios exercise the DBSP circuit via a headless Bevy app and use
//! `rust-rspec` to express expectations declaratively. The DBSP circuit is the
//! sole source of truth for inferred motion; Bevy merely applies its outputs.

use approx::assert_relative_eq;
use bevy::prelude::*;
use lille::{
    apply_ground_friction,
    components::{Block, BlockSlope, ForceComp},
    DbspPlugin, DdlogId, Health, VelocityComp, FALL_DAMAGE_SCALE, GRAVITY_PULL, GROUND_FRICTION,
    SAFE_LANDING_SPEED, TERMINAL_VELOCITY,
};
use rstest::{fixture, rstest};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Physics test values remain within f32 representable range"
)]
fn as_f32(value: f64) -> f32 {
    value as f32
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Physics damage calculations clamp to u16 bounds"
)]
fn as_u16(value: f64) -> u16 {
    value as u16
}

#[derive(Clone)]
struct TestWorld {
    /// Shared Bevy app; `rspec` fixtures must implement `Clone + Send + Sync`.
    app: Arc<Mutex<App>>,
    entity: Option<Entity>,
    expected_damage: Arc<Mutex<Option<u16>>>,
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
            app: Arc::new(Mutex::new(app)),
            entity: None,
            expected_damage: Arc::new(Mutex::new(None)),
        }
    }
}

impl TestWorld {
    fn app_guard(&self) -> MutexGuard<'_, App> {
        self.app
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn expected_damage_guard(&self) -> MutexGuard<'_, Option<u16>> {
        self.expected_damage
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn entity_or_panic(&self) -> Entity {
        self.entity.unwrap_or_else(|| panic!("entity not spawned"))
    }

    /// Spawns a block into the world.
    fn spawn_block(&mut self, block: Block) {
        self.app_guard().world.spawn(block);
    }

    /// Spawns a block together with its slope on the same entity.
    fn spawn_sloped_block(&mut self, block: Block, slope: BlockSlope) {
        self.app_guard().world.spawn((block, slope));
    }

    /// Spawns an entity at `transform` with the supplied velocity.
    fn spawn_entity(&mut self, transform: Transform, vel: VelocityComp, force: Option<ForceComp>) {
        let mut app = self.app_guard();
        let mut entity = app.world.spawn((DdlogId(1), transform, vel));
        if let Some(force_comp) = force {
            entity.insert(force_comp);
        }
        self.entity = Some(entity.id());
    }

    /// Spawns an entity without an external force.
    fn spawn_entity_without_force(&mut self, transform: Transform, vel: VelocityComp) {
        self.spawn_entity(transform, vel, None);
    }

    /// Spawns an entity with an attached health component.
    fn spawn_entity_with_health(
        &mut self,
        transform: Transform,
        vel: VelocityComp,
        health: Health,
    ) {
        let mut app = self.app_guard();
        let entity = app.world.spawn((DdlogId(1), transform, vel, health));
        self.entity = Some(entity.id());
    }

    fn health(&self) -> Health {
        let app = self.app_guard();
        let entity = self.entity_or_panic();
        app.world
            .get::<Health>(entity)
            .cloned()
            .unwrap_or_else(|| panic!("missing Health component"))
    }

    fn set_position_z(&self, z: f32) {
        let mut app = self.app_guard();
        let entity = self.entity_or_panic();
        let Some(mut transform) = app.world.get_mut::<Transform>(entity) else {
            panic!("missing Transform component");
        };
        transform.translation.z = z;
    }

    fn set_velocity_z(&self, vz: f32) {
        let mut app = self.app_guard();
        let entity = self.entity_or_panic();
        let Some(mut velocity) = app.world.get_mut::<VelocityComp>(entity) else {
            panic!("missing VelocityComp component");
        };
        velocity.vz = vz;
    }

    fn set_expected_damage(&self, damage: u16) {
        *self.expected_damage_guard() = Some(damage);
    }

    fn take_expected_damage(&self) -> u16 {
        let mut expected = self.expected_damage_guard();
        expected
            .take()
            .unwrap_or_else(|| panic!("expected damage should be recorded"))
    }

    /// Advances the simulation by one tick.
    fn tick(&mut self) {
        self.app_guard().update();
    }

    /// Generic assertion helper for components with tolerance checking.
    fn assert_component_values<T, F>(&self, name: &str, extract: F, expected: &[f32])
    where
        T: Component,
        F: Fn(&T) -> Vec<f32>,
    {
        let app = self.app_guard();
        let entity = self.entity_or_panic();
        let Some(component) = app.world.get::<T>(entity) else {
            panic!("missing {name}");
        };

        let actual = extract(component);
        let tolerance = 1e-3;

        for (a, e) in actual.iter().zip(expected.iter()) {
            assert_relative_eq!(*a, *e, epsilon = tolerance);
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

/// Provides a fresh Bevy world for each scenario.
#[fixture]
fn world() -> TestWorld {
    TestWorld::default()
}

/// Runs a physics scenario using `rspec` with the provided parameters.
macro_rules! physics_spec {
    ($world:expr, $description:expr, $setup:expr, $expected_pos:expr, $expected_vel:expr) => {
        rspec::run(&rspec::given($description, ($world), |scenario| {
            scenario.before_each($setup);
            scenario.when("the simulation ticks once", |phase| {
                phase.before_each(|world_state| world_state.tick());
                phase.then("the expected outcome occurs", move |world_state| {
                    world_state
                        .assert_position(($expected_pos).0, ($expected_pos).1, ($expected_pos).2);
                    world_state
                        .assert_velocity(($expected_vel).0, ($expected_vel).1, ($expected_vel).2);
                });
            });
        }));
    };
}

#[rstest]
#[case::falling(
    "an unsupported entity",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: -2 });
          world.spawn_entity_without_force(Transform::from_xyz(0.0, 0.0, 2.0), VelocityComp::default());
      },
    (0.0, 0.0, 1.0),
    (0.0, 0.0, GRAVITY_PULL as f32)
)]
#[case::standing_flat(
    "an entity on a flat block",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
          world.spawn_entity_without_force(Transform::from_xyz(0.0, 0.0, 1.0), VelocityComp::default());
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
          world.spawn_entity_without_force(Transform::from_xyz(0.0, 0.0, 1.5), VelocityComp::default());
      },
    (0.0, 0.0, 1.5),
    (0.0, 0.0, 0.0)
)]
#[case::move_heights(
    "an entity moving across blocks of different heights",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
          world.spawn_block(Block { id: 2, x: 1, y: 0, z: 1 });
          world.spawn_entity_without_force(
              Transform::from_xyz(0.0, 0.0, 1.0),
              VelocityComp { vx: 1.0 / (1.0 - GROUND_FRICTION as f32), vy: 0.0, vz: 0.0 },
          );
      },
    (
        apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)) as f32,
        0.0,
        2.0,
    ),
    (
        apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)) as f32,
        0.0,
        0.0,
    )
)]
#[case::force_acceleration(
    "an entity accelerates under force",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
          world.spawn_block(Block { id: 2, x: 1, y: 0, z: 1 });
          world.spawn_entity(
              Transform::from_xyz(0.0, 0.0, 1.0),
              VelocityComp::default(),
              Some(ForceComp { force_x: 5.0 / (1.0 - GROUND_FRICTION), force_y: 0.0, force_z: 0.0, mass: Some(5.0) }),
          );
      },
    (
        apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)) as f32,
        0.0,
        2.0,
    ),
    (
        apply_ground_friction(1.0 / (1.0 - GROUND_FRICTION)) as f32,
        0.0,
        0.0,
    )
)]
#[case::force_mass_z(
    "an unsupported entity accelerates along Z",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: -2 });
          world.spawn_entity(
              Transform::from_xyz(0.0, 0.0, 2.0),
              VelocityComp::default(),
              Some(ForceComp { force_x: 0.0, force_y: 0.0, force_z: 10.0, mass: Some(5.0) }),
          );
      },
    (0.0, 0.0, 3.0),
    (0.0, 0.0, 1.0)
)]
#[case::invalid_mass(
    "a force with invalid mass is ignored",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: -2 });
          world.spawn_entity(
              Transform::from_xyz(0.0, 0.0, 2.0),
              VelocityComp::default(),
              Some(ForceComp { force_x: 0.0, force_y: 0.0, force_z: 10.0, mass: Some(0.0) }),
          );
      },
    (0.0, 0.0, 1.0),
    (0.0, 0.0, GRAVITY_PULL as f32)
)]
#[case::standing_friction(
    "a standing entity slows due to friction",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
          world.spawn_entity_without_force(
              Transform::from_xyz(0.0, 0.0, 1.0),
              VelocityComp { vx: 1.0, vy: 0.0, vz: 0.0 },
          );
      },
    (
        apply_ground_friction(1.0) as f32,
        0.0,
        1.0,
    ),
    (
        apply_ground_friction(1.0) as f32,
        0.0,
        0.0,
    )
)]
#[case::diagonal_friction(
    "a standing entity with diagonal movement slows due to friction",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: 0 });
          world.spawn_entity_without_force(
              Transform::from_xyz(0.0, 0.0, 1.0),
              VelocityComp { vx: 1.0, vy: 1.0, vz: 0.0 },
          );
      },
    (
        apply_ground_friction(1.0) as f32,
        apply_ground_friction(1.0) as f32,
        1.0,
    ),
    (
        apply_ground_friction(1.0) as f32,
        apply_ground_friction(1.0) as f32,
        0.0,
    )
)]
#[case::force_respects_terminal_velocity(
    "a downward force cannot exceed terminal velocity",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: -10 });
          world.spawn_entity(
              Transform::from_xyz(0.0, 0.0, 5.0),
              VelocityComp::default(),
              Some(ForceComp { force_x: 0.0, force_y: 0.0, force_z: -100.0, mass: Some(5.0) }),
          );
      },
    (0.0, 0.0, 5.0
        + (-20.0 + GRAVITY_PULL as f32)
            .clamp(-TERMINAL_VELOCITY as f32, TERMINAL_VELOCITY as f32)),
    (0.0, 0.0, (-20.0 + GRAVITY_PULL as f32)
        .clamp(-TERMINAL_VELOCITY as f32, TERMINAL_VELOCITY as f32))
)]
#[case::unsupported_velocity_capped(
    "an unsupported entity's fall speed is capped",
      |world: &mut TestWorld| {
          world.spawn_block(Block { id: 1, x: 0, y: 0, z: -10 });
          world.spawn_entity_without_force(
              Transform::from_xyz(0.0, 0.0, 5.0),
              VelocityComp { vx: 0.0, vy: 0.0, vz: -5.0 },
          );
      },
    (0.0, 0.0, 5.0
        + (-5.0 + GRAVITY_PULL as f32)
            .clamp(-TERMINAL_VELOCITY as f32, TERMINAL_VELOCITY as f32)),
    (0.0, 0.0, (-5.0 + GRAVITY_PULL as f32)
        .clamp(-TERMINAL_VELOCITY as f32, TERMINAL_VELOCITY as f32))
)]
fn physics_scenarios(
    world: TestWorld,
    #[case] description: &'static str,
    #[case] setup: fn(&mut TestWorld),
    #[case] expected_pos: (f32, f32, f32),
    #[case] expected_vel: (f32, f32, f32),
) {
    physics_spec!(world, description, setup, expected_pos, expected_vel);
}

#[rstest]
fn falling_inflicts_health_damage(world: TestWorld) {
    rspec::run(&rspec::given(
        "an entity falling onto level ground",
        world,
        |scenario| {
            scenario.before_each(|world| {
                world.spawn_block(Block {
                    id: 99,
                    x: 0,
                    y: 0,
                    z: 0,
                });
                world.spawn_entity_with_health(
                    Transform::from_xyz(0.0, 0.0, 10.0),
                    VelocityComp::default(),
                    Health {
                        current: 100,
                        max: 100,
                    },
                );
            });
            scenario.when("the simulation runs until the entity lands", |phase| {
                phase.before_each(|world_state| {
                    let fall_speed = -(as_f32(SAFE_LANDING_SPEED) + 4.0);
                    world_state.set_velocity_z(fall_speed);
                    world_state.tick();

                    world_state.set_velocity_z(0.0);
                    world_state.set_position_z(1.0);
                    world_state.tick();

                    let impact_speed = f64::from(-(fall_speed + as_f32(GRAVITY_PULL)))
                        .clamp(0.0, TERMINAL_VELOCITY);
                    let excess = impact_speed - SAFE_LANDING_SPEED;
                    let expected_damage = if excess <= 0.0 {
                        0
                    } else {
                        as_u16((excess * FALL_DAMAGE_SCALE).min(f64::from(u16::MAX)).floor())
                    };
                    world_state.set_expected_damage(expected_damage);
                });
                phase.then("the expected fall damage is applied", |world_state| {
                    let expected = world_state.take_expected_damage();
                    let health = world_state.health();
                    let lost = 100u16.saturating_sub(health.current);
                    assert_eq!(lost, expected);
                });
            });
        },
    ));
}
