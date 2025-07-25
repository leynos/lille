use bevy::prelude::*;
use lille::{DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL};

/// Verifies that the ECS-DBSP round trip applies gravity to entity position and
/// velocity.

#[test]
fn ecs_dbsp_round_trip_applies_gravity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);

    let entity = app
        .world
        .spawn((
            DdlogId(1),
            Transform::from_xyz(0.0, 0.0, 1.0),
            VelocityComp::default(),
        ))
        .id();

    app.update();

    let transform = app.world.get::<Transform>(entity).unwrap();
    assert!((transform.translation.z - (1.0 + GRAVITY_PULL as f32)).abs() < f32::EPSILON);

    let vel = app.world.get::<VelocityComp>(entity).unwrap();
    assert!((vel.vz - GRAVITY_PULL as f32).abs() < f32::EPSILON);
}
