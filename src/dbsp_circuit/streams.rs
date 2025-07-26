//! Helper functions for constructing DBSP dataflow streams.

use dbsp::{operator::Max, typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;

use crate::components::{Block, BlockSlope};
use crate::{BLOCK_CENTRE_OFFSET, BLOCK_TOP_OFFSET, GRAVITY_PULL};

use super::{FloorHeightAt, HighestBlockAt, Position, Velocity};

/// Returns a stream pairing each grid cell with its highest block and id.
///
/// The function aggregates incoming [`Block`] records by `(x, y)` to find the
/// maximum `z` value at each coordinate. The output preserves the originating
/// block id so that subsequent joins can access slope information.
///
/// # Examples
///
/// ```rust,ignore
/// # use lille::prelude::*;
/// # use dbsp::{RootCircuit, typed_batch::OrdZSet};
/// let (circuit, _) = RootCircuit::build(|_c| Ok(()))?;
/// let (stream, _handle) = circuit.add_input_zset::<Block>();
/// let _highest = highest_block_pair(&stream);
/// # Ok::<(), dbsp::Error>(())
/// ```
pub(super) fn highest_block_pair(
    blocks: &Stream<RootCircuit, OrdZSet<Block>>,
) -> Stream<RootCircuit, OrdZSet<(HighestBlockAt, i64)>> {
    blocks
        .map_index(|b| ((b.x, b.y), (b.z, b.id)))
        .aggregate(Max)
        .map(|((x, y), (z, id))| {
            (
                HighestBlockAt {
                    x: *x,
                    y: *y,
                    z: *z,
                },
                *id,
            )
        })
}

/// Calculates the world-space `z` coordinate of a block's upper surface.
fn block_top(z: i32) -> f64 {
    z as f64 + BLOCK_TOP_OFFSET
}

/// Derives the floor height for each block, optionally applying slopes.
///
/// The stream joins the highest block id at a grid cell with any matching
/// [`BlockSlope`] record. When slope data is present the returned
/// [`FloorHeightAt`] accounts for the block's gradient, producing a smooth
/// surface. Missing slope data falls back to a flat top.
pub(super) fn floor_height_stream(
    highest_pair: &Stream<RootCircuit, OrdZSet<(HighestBlockAt, i64)>>,
    slopes: &Stream<RootCircuit, OrdZSet<BlockSlope>>,
) -> Stream<RootCircuit, OrdZSet<FloorHeightAt>> {
    highest_pair
        .map_index(|(hb, id)| (*id, (hb.x, hb.y, hb.z)))
        .outer_join(
            &slopes.map_index(|bs| (bs.block_id, (bs.grad_x, bs.grad_y))),
            |_, &(x, y, z), &(grad_x, grad_y)| {
                let block_top = block_top(z);
                Some(FloorHeightAt {
                    x,
                    y,
                    z: OrderedFloat(
                        block_top
                            + BLOCK_CENTRE_OFFSET * grad_x.into_inner()
                            + BLOCK_CENTRE_OFFSET * grad_y.into_inner(),
                    ),
                })
            },
            |_, &(x, y, z)| {
                let block_top = block_top(z);
                Some(FloorHeightAt {
                    x,
                    y,
                    z: OrderedFloat(block_top),
                })
            },
            |_, _| None,
        )
        .flat_map(|fh| fh.clone().into_iter())
}

/// Applies gravity to each velocity record.
///
/// This helper keeps the velocity update logic separate from entity position
/// updates. It adds [`GRAVITY_PULL`] to the incoming `vz` component and passes
/// through the remaining fields unchanged.
pub(super) fn new_velocity_stream(
    velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
) -> Stream<RootCircuit, OrdZSet<Velocity>> {
    velocities.map(|v| Velocity {
        entity: v.entity,
        vx: v.vx,
        vy: v.vy,
        vz: OrderedFloat(v.vz.into_inner() + GRAVITY_PULL),
    })
}

/// Integrates positions with updated velocities.
///
/// The input streams are joined by `entity`, producing a new [`Position`]
/// translated by the entity's velocity components. The function performs a
/// simple Euler integration suitable for the small time step used in the game
/// loop.
pub(super) fn new_position_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    new_vel: &Stream<RootCircuit, OrdZSet<Velocity>>,
) -> Stream<RootCircuit, OrdZSet<Position>> {
    let joined = positions.map_index(|p| (p.entity, p.clone())).join(
        &new_vel.map_index(|v| (v.entity, v.clone())),
        |_, pos, vel| (pos.clone(), vel.clone()),
    );

    joined.map(|(p, v)| Position {
        entity: p.entity,
        x: OrderedFloat(p.x.into_inner() + v.vx.into_inner()),
        y: OrderedFloat(p.y.into_inner() + v.vy.into_inner()),
        z: OrderedFloat(p.z.into_inner() + v.vz.into_inner()),
    })
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    size_of::SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct PositionFloor {
    pub position: Position,
    pub z_floor: OrderedFloat<f64>,
}

/// Joins each `Position` with the corresponding floor height.
///
/// Positions are discretised to grid coordinates by flooring their `x` and `y`
/// values. Those indices look up a [`FloorHeightAt`] record to produce a
/// [`PositionFloor`] stream suitable for higher-level physics logic.
pub(super) fn position_floor_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    floor_height: &Stream<RootCircuit, OrdZSet<FloorHeightAt>>,
) -> Stream<RootCircuit, OrdZSet<PositionFloor>> {
    positions
        .map_index(|p| {
            (
                (
                    p.x.into_inner().floor() as i32,
                    p.y.into_inner().floor() as i32,
                ),
                p.clone(),
            )
        })
        .join(
            &floor_height.map_index(|fh| ((fh.x, fh.y), fh.z)),
            |_, pos, z| PositionFloor {
                position: pos.clone(),
                z_floor: *z,
            },
        )
}
