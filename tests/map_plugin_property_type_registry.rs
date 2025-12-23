#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering map custom property type registration."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Verifies that `LilleMapPlugin` registers Tiled custom property types.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use std::any::TypeId;

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::{Reflect, TypeRegistry};
use lille::map::{Collidable, PlayerSpawn, SlopeProperties, SpawnPoint};
use lille::LilleMapPlugin;
use rstest::rstest;

fn is_type_registered_as_component(registry: &TypeRegistry, type_id: TypeId) -> bool {
    registry
        .get(type_id)
        .and_then(|entry| entry.data::<ReflectComponent>())
        .is_some()
}

/// Dummy type to verify that unregistered types return false.
#[derive(Reflect)]
struct NotRegistered;

#[rstest]
#[case::collidable(TypeId::of::<Collidable>(), "Collidable")]
#[case::slope_properties(TypeId::of::<SlopeProperties>(), "SlopeProperties")]
#[case::player_spawn(TypeId::of::<PlayerSpawn>(), "PlayerSpawn")]
#[case::spawn_point(TypeId::of::<SpawnPoint>(), "SpawnPoint")]
fn registers_map_property_type(#[case] type_id: TypeId, #[case] type_name: &str) {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.add_plugins(LilleMapPlugin);

    let registry = app.world().resource::<AppTypeRegistry>();
    let registry_guard = registry.read();

    assert!(
        is_type_registered_as_component(&registry_guard, type_id),
        "{type_name} should be registered as a component"
    );
}

#[rstest]
fn unregistered_type_is_not_a_component() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.add_plugins(LilleMapPlugin);

    let registry = app.world().resource::<AppTypeRegistry>();
    let registry_guard = registry.read();

    assert!(
        !is_type_registered_as_component(&registry_guard, TypeId::of::<NotRegistered>()),
        "NotRegistered should not be registered as a component"
    );
}
