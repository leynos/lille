use bevy::prelude::*;
use cucumber::{given, then, when};
use lille::{DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL};

#[derive(Debug, Default, cucumber::World)]
pub struct PhysicsWorld {
    app: App,
    entity: Option<Entity>,
}

#[given("a headless app with a single unsupported entity")]
fn given_headless_app(world: &mut PhysicsWorld) {
    world.app = App::new();
    world
        .app
        .add_plugins(MinimalPlugins)
        .add_plugins(DbspPlugin);

    let id = world
        .app
        .world
        .spawn((
            DdlogId(1),
            Transform::from_xyz(0.0, 0.0, 1.0),
            VelocityComp::default(),
        ))
        .id();
    world.entity = Some(id);
}

#[when("the simulation ticks once")]
fn when_tick(world: &mut PhysicsWorld) {
    world.app.update();
}

#[then(expr = "the entity's z position should be {float}")]
fn then_check_z(world: &mut PhysicsWorld, expected_z: f32) {
    let entity = world.entity.expect("entity not spawned");
    let transform = world.app.world.get::<Transform>(entity).unwrap();
    let actual_z = transform.translation.z;
    assert!(
        (actual_z - expected_z).abs() < f32::EPSILON,
        "expected z {expected_z}, got {actual_z}"
    );
    // Ensure velocity was updated by gravity as well
    let vel = world.app.world.get::<VelocityComp>(entity).unwrap();
    assert!((vel.vz - GRAVITY_PULL as f32).abs() < f32::EPSILON);
}
