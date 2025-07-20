//! Cucumber step definitions for gravity physics simulation tests.
//!
//! This module provides BDD steps that verify the correct application of
//! gravity to entities within a headless Bevy application using the
//! `DbspPlugin`.

use bevy::prelude::*;
use cucumber::{given, then, when};
use lille::{DbspPlugin, DdlogId, VelocityComp, GRAVITY_PULL};

#[derive(Debug, Default, cucumber::World)]
pub struct PhysicsWorld {
    app: App,
    entity: Option<Entity>,
}

#[given("a headless app with a single unsupported entity")]
async fn given_headless_app(world: &mut PhysicsWorld) {
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
async fn when_tick(world: &mut PhysicsWorld) {
    // DBSP uses its own Tokio runtime internally. Running `update` directly
    // inside the async context would nest runtimes and panic. `block_in_place`
    // executes the update on a dedicated thread, avoiding the conflict.
    // DBSP spawns its own Tokio runtime and panics if `block_on` is called
    // within an existing runtime. Running Bevy's update on a dedicated thread
    // sidesteps this restriction without requiring additional async juggling.
    let mut app = std::mem::replace(&mut world.app, App::new());
    world.app = std::thread::spawn(move || {
        app.update();
        app
    })
    .join()
    .expect("update thread panicked");
}

#[then(expr = "the entity's z position should be {float}")]
async fn then_check_z(world: &mut PhysicsWorld, expected_z: f32) {
    let entity = world.entity.expect("entity not spawned");
    let transform = world
        .app
        .world
        .get::<Transform>(entity)
        .expect("entity should have Transform component");
    let actual_z = transform.translation.z;
    let tolerance = 1e-3;
    assert!(
        (actual_z - expected_z).abs() < tolerance,
        "expected z {expected_z}, got {actual_z}"
    );
    // Ensure velocity was updated by gravity as well
    let vel = world
        .app
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
