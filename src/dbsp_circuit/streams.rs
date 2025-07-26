//! Helper functions for constructing DBSP dataflow streams.

use dbsp::{operator::Max, typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;

use crate::components::{Block, BlockSlope};
use crate::{BLOCK_CENTRE_OFFSET, BLOCK_TOP_OFFSET, GRAVITY_PULL};

use super::{FloorHeightAt, HighestBlockAt, Position, Velocity};

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

fn block_top(z: i32) -> f64 {
    z as f64 + BLOCK_TOP_OFFSET
}

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
