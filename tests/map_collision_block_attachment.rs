#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering Block attachment to Collidable tiles."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Verifies that `LilleMapPlugin` attaches `Block` components to `Collidable` entities.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TilePos, TiledEvent};
use lille::components::Block;
use lille::map::Collidable;
use lille::LilleMapPlugin;
use rstest::{fixture, rstest};

/// Creates a minimal Bevy app configured for block attachment testing.
///
/// The app includes map test plugins and the map plugin, but does not load any
/// map assets. This allows spawning mock entities directly for unit testing.
#[fixture]
fn test_app() -> App {
    let mut app = App::new();
    map_test_plugins::add_map_test_plugins(&mut app);
    app.add_plugins(LilleMapPlugin);
    app
}

/// Spawns a tile entity with `Collidable` and `TilePos` components.
fn spawn_collidable_tile(world: &mut World, x: u32, y: u32) -> Entity {
    world.spawn((Collidable, TilePos { x, y })).id()
}

/// Spawns a tile entity with only `TilePos` (no `Collidable`).
fn spawn_non_collidable_tile(world: &mut World, x: u32, y: u32) -> Entity {
    world.spawn(TilePos { x, y }).id()
}

/// Triggers a `MapCreated` event in the world.
#[expect(deprecated, reason = "bevy_ecs_tiled 0.10 uses the legacy Event API.")]
fn trigger_map_created(world: &mut World) {
    world.send_event(TiledEvent::new(Entity::PLACEHOLDER, MapCreated));
}

#[rstest]
fn attaches_block_to_collidable_entity(mut test_app: App) {
    let entity = spawn_collidable_tile(test_app.world_mut(), 5, 10);
    trigger_map_created(test_app.world_mut());

    // Run systems to process the event.
    test_app.update();

    let block = test_app
        .world()
        .get::<Block>(entity)
        .expect("expected Block component on collidable entity");

    assert_eq!(block.x, 5, "block x should match TilePos x");
    assert_eq!(block.y, 10, "block y should match TilePos y");
    assert_eq!(block.z, 0, "block z should be 0 for single-level maps");
}

#[rstest]
fn does_not_attach_block_to_non_collidable_entity(mut test_app: App) {
    let entity = spawn_non_collidable_tile(test_app.world_mut(), 3, 7);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    assert!(
        test_app.world().get::<Block>(entity).is_none(),
        "non-collidable entities should not receive Block"
    );
}

#[rstest]
fn is_idempotent(mut test_app: App) {
    let entity = spawn_collidable_tile(test_app.world_mut(), 2, 4);

    // First map load event.
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let first_block = test_app
        .world()
        .get::<Block>(entity)
        .expect("expected Block after first update")
        .clone();

    // Second map load event (simulating a map reload).
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let second_block = test_app
        .world()
        .get::<Block>(entity)
        .expect("expected Block after second update");

    // Block should be unchanged (same ID, same coordinates).
    assert_eq!(
        first_block.id, second_block.id,
        "block ID should not change on second run"
    );
}

#[rstest]
fn assigns_unique_block_ids(mut test_app: App) {
    let entity1 = spawn_collidable_tile(test_app.world_mut(), 0, 0);
    let entity2 = spawn_collidable_tile(test_app.world_mut(), 1, 0);
    let entity3 = spawn_collidable_tile(test_app.world_mut(), 0, 1);

    trigger_map_created(test_app.world_mut());
    test_app.update();

    let block1 = test_app
        .world()
        .get::<Block>(entity1)
        .expect("entity1 should have Block");
    let block2 = test_app
        .world()
        .get::<Block>(entity2)
        .expect("entity2 should have Block");
    let block3 = test_app
        .world()
        .get::<Block>(entity3)
        .expect("entity3 should have Block");

    assert_ne!(block1.id, block2.id, "block IDs should be unique");
    assert_ne!(block2.id, block3.id, "block IDs should be unique");
    assert_ne!(block1.id, block3.id, "block IDs should be unique");
}

#[rstest]
#[case::origin(0, 0)]
#[case::positive(100, 200)]
#[case::large(1000, 2000)]
fn block_coordinates_match_tile_pos(mut test_app: App, #[case] tile_x: u32, #[case] tile_y: u32) {
    let entity = spawn_collidable_tile(test_app.world_mut(), tile_x, tile_y);
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let block = test_app
        .world()
        .get::<Block>(entity)
        .expect("expected Block component");

    #[expect(
        clippy::cast_possible_wrap,
        reason = "Test values are small and fit in i32."
    )]
    {
        assert_eq!(block.x, tile_x as i32);
        assert_eq!(block.y, tile_y as i32);
    }
}
