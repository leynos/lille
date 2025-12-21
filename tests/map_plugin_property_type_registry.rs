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

fn is_component_registered<T: Reflect>(registry: &TypeRegistry) -> bool {
    registry
        .get(TypeId::of::<T>())
        .and_then(|entry| entry.data::<ReflectComponent>())
        .is_some()
}

#[rstest]
fn registers_map_property_types() {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.add_plugins(LilleMapPlugin);

    let registry = app.world().resource::<AppTypeRegistry>();
    let registry_guard = registry.read();

    assert_eq!(is_component_registered::<Collidable>(&registry_guard), true);
    assert_eq!(
        is_component_registered::<SlopeProperties>(&registry_guard),
        true
    );
    assert_eq!(
        is_component_registered::<PlayerSpawn>(&registry_guard),
        true
    );
    assert_eq!(is_component_registered::<SpawnPoint>(&registry_guard), true);
}
