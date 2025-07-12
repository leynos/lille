use bevy::prelude::*;
use lille::{
    apply_dbsp_outputs_system, cache_state_for_dbsp_system, init_dbsp_system, DdlogId,
    VelocityComp as Velocity, GRAVITY_PULL,
};

#[test]
fn ecs_dbsp_round_trip_applies_gravity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, init_dbsp_system)
        .add_systems(
            Update,
            (cache_state_for_dbsp_system, apply_dbsp_outputs_system).chain(),
        );

    let entity = app
        .world
        .spawn((
            DdlogId(1),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Velocity::default(),
        ))
        .id();

    app.update();

    let transform = app.world.get::<Transform>(entity).unwrap();
    assert!((transform.translation.z - (1.0 + GRAVITY_PULL as f32)).abs() < f32::EPSILON);

    let vel = app.world.get::<Velocity>(entity).unwrap();
    assert!((vel.vz - GRAVITY_PULL as f32).abs() < f32::EPSILON);
}
