//! DBSP dataflow stream construction for spatial simulation.
//!
//! This module defines helper functions for building the dataflow streams used
//! by `DbspCircuit` to process the game world. These streams implement:
//!
//! - Block aggregation to track the highest block at each grid cell
//! - Floor height calculation with optional slopes
//! - Velocity updates applying gravity
//! - Position integration based on velocities
//! - Joining positions with floor height for collision queries
//!
//! Each function returns a new [`Stream`] that can be composed into the overall
//! circuit.

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

/// Calculates the world-space `z` coordinate for a block's floor height.
///
/// If a slope gradient is provided, it adjusts the block's top height by the
/// gradient scaled by [`BLOCK_CENTRE_OFFSET`]. This helper keeps the
/// slope-adjustment logic out of the stream closures so that they remain
/// concise.
fn compute_floor_height_at(x: i32, y: i32, z: i32, grad: Option<(f64, f64)>) -> FloorHeightAt {
    let base = block_top(z);
    let extra = grad
        .map(|(gx, gy)| BLOCK_CENTRE_OFFSET * (gx + gy))
        .unwrap_or(0.0);
    FloorHeightAt {
        x,
        y,
        z: OrderedFloat(base + extra),
    }
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
    let slope_idx = slopes.map_index(|bs| (bs.block_id, (bs.grad_x, bs.grad_y)));
    highest_pair
        .map_index(|(hb, id)| (*id, (hb.x, hb.y, hb.z)))
        .outer_join(
            &slope_idx,
            |_, &(x, y, z), &(gx, gy)| {
                Some(compute_floor_height_at(
                    x,
                    y,
                    z,
                    Some((gx.into_inner(), gy.into_inner())),
                ))
            },
            |_, &(x, y, z)| Some(compute_floor_height_at(x, y, z, None)),
            |_, _| None,
        )
        .flat_map(|fh| fh.clone())
}

/// Updates velocities by applying gravitational acceleration.
///
/// The stream adds [`GRAVITY_PULL`] to each entity's vertical velocity
/// component while leaving the horizontal components unchanged. This helper
/// keeps the gravity logic separate from position integration.
///
/// # Examples
///
/// ```rust,ignore
/// # use lille::prelude::*;
/// # use lille::dbsp_circuit::{self, Velocity};
/// # use dbsp::{RootCircuit, typed_batch::OrdZSet};
/// let (circuit, (vel_stream, _)) = RootCircuit::build(|c| {
///     Ok(c.add_input_zset::<Velocity>())
/// })?;
/// let _updated = new_velocity_stream(&vel_stream);
/// # Ok::<(), dbsp::Error>(())
/// ```
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
    positions.map_index(|p| (p.entity, p.clone())).join(
        &new_vel.map_index(|v| (v.entity, v.clone())),
        |_, p, v| Position {
            entity: p.entity,
            x: OrderedFloat(p.x.into_inner() + v.vx.into_inner()),
            y: OrderedFloat(p.y.into_inner() + v.vy.into_inner()),
            z: OrderedFloat(p.z.into_inner() + v.vz.into_inner()),
        },
    )
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
/// Pairs an entity's position with the floor height at its grid location.
///
/// This struct is the result of joining the continuous [`Position`] stream with
/// the discrete [`FloorHeightAt`] grid. It is primarily consumed by higher-level
/// physics logic for tasks such as collision detection or standing vs. falling
/// checks.
///
/// * `position` - The entity's current position in world coordinates
/// * `z_floor` - The computed floor height at the position's grid cell
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
            |_idx, pos, &z_floor| PositionFloor {
                position: pos.clone(),
                z_floor,
            },
        )
}
