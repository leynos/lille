//! Exercises the ECS ↔ DBSP loop to ensure gravity persists through a full tick.

use approx::assert_relative_eq;
use bevy::prelude::*;
use lille::{components::Block, DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL};

#[test]
fn ecs_dbsp_round_trip_applies_gravity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);

    app.world.spawn(Block {
        id: 1,
        x: 0,
        y: 0,
        z: 0,
    });

    let entity = app
        .world
        .spawn((
            DdlogId(1),
            Transform::from_xyz(0.0, 0.0, 2.0),
            VelocityComp::default(),
        ))
        .id();

    app.update();

    let transform = app
        .world
        .get::<Transform>(entity)
        .expect("Transform component should persist after DBSP round trip");
    assert_relative_eq!(f64::from(transform.translation.z), 2.0 + GRAVITY_PULL);

    let vel = app
        .world
        .get::<VelocityComp>(entity)
        .expect("Velocity component should persist after DBSP round trip");
    assert_relative_eq!(f64::from(vel.vz), GRAVITY_PULL);
}
