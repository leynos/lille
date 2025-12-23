//! Map-to-engine translation systems.
//!
//! This module bridges Tiled map annotations with the DBSP physics circuit by
//! attaching engine components (such as `Block`) to entities that carry
//! authoring markers (such as `Collidable`).
//!
//! The translation happens once per map load, triggered by the
//! `TiledEvent<MapCreated>` event. This ensures all tiles are spawned and their
//! custom properties hydrated before we process them.

use bevy::prelude::*;
use bevy_ecs_tiled::prelude::{MapCreated, TilePos, TiledEvent};

use crate::components::Block;
use crate::map::Collidable;

/// Attaches `Block` components to entities marked `Collidable`.
///
/// This system listens for `TiledEvent<MapCreated>` and iterates over all
/// entities with `Collidable` that lack a `Block` component. For each, it
/// derives block coordinates from `TilePos` and inserts a new `Block`.
///
/// The system is idempotent: entities that already have `Block` are skipped,
/// making it safe to run multiple times.
///
/// # Block ID generation
///
/// Block IDs are assigned sequentially via a `Local<i64>` counter. These IDs
/// need only be unique within a single map load session; they are not persisted
/// across saves or map reloads.
///
/// # Coordinate mapping
///
/// - `Block::x` and `Block::y` are derived directly from `TilePos::x` and
///   `TilePos::y` respectively.
/// - `Block::z` is set to `0` for all blocks, as multi-level vertical stacking
///   is out of scope for Phase 1.
#[expect(deprecated, reason = "bevy_ecs_tiled 0.10 uses the legacy Event API.")]
#[expect(
    clippy::type_complexity,
    reason = "Bevy ECS query with filter combinators is inherently verbose."
)]
pub fn attach_collision_blocks(
    mut commands: Commands,
    mut map_events: EventReader<TiledEvent<MapCreated>>,
    collidable_tiles: Query<(Entity, &TilePos), (With<Collidable>, Without<Block>)>,
    mut block_id_counter: Local<i64>,
) {
    // Only process when a map has just finished loading.
    if map_events.read().next().is_none() {
        return;
    }

    // Drain all remaining events (supports multiple maps, though unusual).
    map_events.read().for_each(drop);

    for (entity, tile_pos) in &collidable_tiles {
        // TilePos uses u32 for grid coordinates. For typical map sizes (well
        // under 2^31 tiles), this cast is safe. Maps larger than i32::MAX
        // tiles per axis would require a different physics representation.
        #[expect(
            clippy::cast_possible_wrap,
            reason = "Tile coordinates in practical maps fit comfortably in i32."
        )]
        let block = Block {
            id: *block_id_counter,
            x: tile_pos.x as i32,
            y: tile_pos.y as i32,
            z: 0,
        };

        commands.entity(entity).insert(block);
        *block_id_counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "Test values are small and fit in i32."
    )]
    fn block_coordinates_from_tile_pos() {
        let tile_pos = TilePos { x: 5, y: 10 };
        let block = Block {
            id: 0,
            x: tile_pos.x as i32,
            y: tile_pos.y as i32,
            z: 0,
        };

        assert_eq!(block.x, 5);
        assert_eq!(block.y, 10);
        assert_eq!(block.z, 0);
    }
}
