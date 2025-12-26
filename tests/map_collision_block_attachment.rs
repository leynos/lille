#![cfg_attr(
    feature = "test-support",
    doc = "Unit tests covering Block and `BlockSlope` attachment to tiles."
)]
#![cfg_attr(not(feature = "test-support"), doc = "Tests require `test-support`.")]
#![cfg(feature = "test-support")]
//! Verifies that `LilleMapPlugin` attaches `Block` and `BlockSlope` components to
//! `Collidable` entities, with `BlockSlope` only attached when `SlopeProperties` is present.

#[path = "support/map_test_plugins.rs"]
mod map_test_plugins;

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TilePos, TiledEvent};
use lille::components::{Block, BlockSlope};
use lille::map::{Collidable, SlopeProperties};
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

/// Spawns a tile entity with `SlopeProperties` and `TilePos` but no `Collidable`.
fn spawn_sloped_non_collidable_tile(
    world: &mut World,
    x: u32,
    y: u32,
    slope: SlopeProperties,
) -> Entity {
    world.spawn((TilePos { x, y }, slope)).id()
}

/// Spawns a tile entity with `Collidable`, `TilePos`, and `SlopeProperties`.
fn spawn_sloped_collidable_tile(
    world: &mut World,
    x: u32,
    y: u32,
    slope: SlopeProperties,
) -> Entity {
    world.spawn((Collidable, TilePos { x, y }, slope)).id()
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
fn does_not_attach_block_or_slope_to_sloped_non_collidable_entity(mut test_app: App) {
    let slope = SlopeProperties {
        grad_x: 0.25,
        grad_y: 0.5,
    };
    let entity = spawn_sloped_non_collidable_tile(test_app.world_mut(), 3, 7, slope);
    trigger_map_created(test_app.world_mut());

    test_app.update();

    assert!(
        test_app.world().get::<Block>(entity).is_none(),
        "tiles with SlopeProperties but without Collidable should not receive Block"
    );
    assert!(
        test_app.world().get::<BlockSlope>(entity).is_none(),
        "tiles with SlopeProperties but without Collidable should not receive BlockSlope"
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

// --- BlockSlope attachment tests ---

#[rstest]
fn attaches_block_slope_to_sloped_entity(mut test_app: App) {
    let slope = SlopeProperties {
        grad_x: 0.25,
        grad_y: 0.5,
    };
    let entity = spawn_sloped_collidable_tile(test_app.world_mut(), 3, 7, slope);
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let block_slope = test_app
        .world()
        .get::<BlockSlope>(entity)
        .expect("expected BlockSlope component on sloped entity");

    assert!(
        (block_slope.grad_x.into_inner() - 0.25).abs() < f64::EPSILON,
        "grad_x should be 0.25"
    );
    assert!(
        (block_slope.grad_y.into_inner() - 0.5).abs() < f64::EPSILON,
        "grad_y should be 0.5"
    );
}

#[rstest]
fn does_not_attach_block_slope_when_no_slope_properties(mut test_app: App) {
    let entity = spawn_collidable_tile(test_app.world_mut(), 5, 10);
    trigger_map_created(test_app.world_mut());
    test_app.update();

    assert!(
        test_app.world().get::<BlockSlope>(entity).is_none(),
        "entities without SlopeProperties should not receive BlockSlope"
    );
}

#[rstest]
fn block_slope_id_matches_block_id(mut test_app: App) {
    let slope = SlopeProperties {
        grad_x: 0.5,
        grad_y: 0.5,
    };
    let entity = spawn_sloped_collidable_tile(test_app.world_mut(), 2, 4, slope);
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let block = test_app
        .world()
        .get::<Block>(entity)
        .expect("expected Block component");
    let block_slope = test_app
        .world()
        .get::<BlockSlope>(entity)
        .expect("expected BlockSlope component");

    assert_eq!(
        block.id, block_slope.block_id,
        "BlockSlope.block_id should match Block.id"
    );
}

#[rstest]
#[case::zero_gradients(0.0, 0.0)]
#[case::positive_gradients(0.25, 0.5)]
#[case::negative_gradients(-0.25, -0.5)]
#[case::unit_gradients(1.0, 1.0)]
#[case::mixed_gradients(-0.5, 0.75)]
fn block_slope_gradients_converted_correctly(
    mut test_app: App,
    #[case] grad_x: f32,
    #[case] grad_y: f32,
) {
    let slope = SlopeProperties { grad_x, grad_y };
    let entity = spawn_sloped_collidable_tile(test_app.world_mut(), 0, 0, slope);
    trigger_map_created(test_app.world_mut());
    test_app.update();

    let block_slope = test_app
        .world()
        .get::<BlockSlope>(entity)
        .expect("expected BlockSlope component");

    let expected_grad_x = f64::from(grad_x);
    let expected_grad_y = f64::from(grad_y);

    assert!(
        (block_slope.grad_x.into_inner() - expected_grad_x).abs() < f64::EPSILON,
        "grad_x conversion mismatch: expected {expected_grad_x}, got {}",
        block_slope.grad_x.into_inner()
    );
    assert!(
        (block_slope.grad_y.into_inner() - expected_grad_y).abs() < f64::EPSILON,
        "grad_y conversion mismatch: expected {expected_grad_y}, got {}",
        block_slope.grad_y.into_inner()
    );
}

#[rstest]
fn multiple_sloped_tiles_have_unique_block_ids(mut test_app: App) {
    let slope1 = SlopeProperties {
        grad_x: 0.25,
        grad_y: 0.0,
    };
    let slope2 = SlopeProperties {
        grad_x: 0.0,
        grad_y: 0.25,
    };
    let slope3 = SlopeProperties {
        grad_x: 0.5,
        grad_y: 0.5,
    };
    let entity1 = spawn_sloped_collidable_tile(test_app.world_mut(), 0, 0, slope1);
    let entity2 = spawn_sloped_collidable_tile(test_app.world_mut(), 1, 0, slope2);
    let entity3 = spawn_sloped_collidable_tile(test_app.world_mut(), 0, 1, slope3);

    trigger_map_created(test_app.world_mut());
    test_app.update();

    let block_slope1 = test_app
        .world()
        .get::<BlockSlope>(entity1)
        .expect("entity1 should have BlockSlope");
    let block_slope2 = test_app
        .world()
        .get::<BlockSlope>(entity2)
        .expect("entity2 should have BlockSlope");
    let block_slope3 = test_app
        .world()
        .get::<BlockSlope>(entity3)
        .expect("entity3 should have BlockSlope");

    assert_ne!(
        block_slope1.block_id, block_slope2.block_id,
        "block_ids should be unique"
    );
    assert_ne!(
        block_slope2.block_id, block_slope3.block_id,
        "block_ids should be unique"
    );
    assert_ne!(
        block_slope1.block_id, block_slope3.block_id,
        "block_ids should be unique"
    );
}
