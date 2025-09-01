//! Floor aggregation and height derivation streams.
//!
//! These helpers process block records to compute discrete floor heights used in
//! movement and collision calculations.

use dbsp::{operator::Max, typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;

use crate::components::{Block, BlockSlope};
use crate::{BLOCK_CENTRE_OFFSET, BLOCK_TOP_OFFSET};

use crate::dbsp_circuit::{FloorHeightAt, HighestBlockAt};

/// Returns a stream pairing each grid cell with its highest block and id.
///
/// The function aggregates incoming [`Block`] records by `(x, y)` to find the
/// maximum `z` value at each coordinate. The output preserves the originating
/// block id so that subsequent joins can access slope information.
///
/// # Examples
///
/// ```rust,no_run
/// # fn main() -> Result<(), dbsp::Error> {
/// # use lille::prelude::*;
/// # use dbsp::{RootCircuit, typed_batch::OrdZSet};
/// # use lille::dbsp_circuit::highest_block_pair;
/// RootCircuit::build(|circuit| {
///     let (stream, _handle) = circuit.add_input_zset::<Block>();
///     let _highest = lille::dbsp_circuit::highest_block_pair(&stream);
///     Ok(())
/// })?;
/// # Ok::<(), dbsp::Error>(())
/// # }
/// ```
pub fn highest_block_pair(
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

/// Derives the floor height for each block, optionally applying slopes.
///
/// The stream joins the highest block id at a grid cell with any matching
/// [`BlockSlope`] record. When slope data is present the returned
/// [`FloorHeightAt`] accounts for the block's gradient, producing a smooth
/// surface. Missing slope data falls back to a flat top.
pub fn floor_height_stream(
    highest_pair: &Stream<RootCircuit, OrdZSet<(HighestBlockAt, i64)>>,
    slopes: &Stream<RootCircuit, OrdZSet<BlockSlope>>,
) -> Stream<RootCircuit, OrdZSet<FloorHeightAt>> {
    highest_pair
        .map_index(|(hb, id)| (*id, (hb.x, hb.y, hb.z)))
        .outer_join(
            &slopes.map_index(|bs| (bs.block_id, (bs.grad_x, bs.grad_y))),
            |_, &(x, y, z), &(gx, gy)| {
                let base = z as f64 + BLOCK_TOP_OFFSET;
                let gradient = BLOCK_CENTRE_OFFSET * (gx.into_inner() + gy.into_inner());
                Some(FloorHeightAt {
                    x,
                    y,
                    z: OrderedFloat(base + gradient),
                })
            },
            |_, &(x, y, z)| {
                Some(FloorHeightAt {
                    x,
                    y,
                    z: OrderedFloat(z as f64 + BLOCK_TOP_OFFSET),
                })
            },
            |_, _| None,
        )
        // Convert `Option<FloorHeightAt>` from the outer join, discarding
        // unmatched slope records.
        .flat_map(|fh| (*fh).into_iter())
}
