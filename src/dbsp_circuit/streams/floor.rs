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
/// ```rust,ignore
/// # use anyhow::Error;
/// # use dbsp::RootCircuit;
/// # use lille::components::Block;
/// # use lille::dbsp_circuit::streams::floor::highest_block_pair;
/// # let _ = RootCircuit::build(|circuit| -> Result<(), Error> {
/// #     let (blocks, _) = circuit.add_input_zset::<Block>();
/// #     let _ = highest_block_pair(&blocks);
/// #     Ok(())
/// # });
/// ```
#[must_use]
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
///
/// # Examples
/// ```rust,ignore
/// # use anyhow::Error;
/// # use dbsp::RootCircuit;
/// # use lille::components::{Block, BlockSlope};
/// # use lille::dbsp_circuit::streams::floor::{floor_height_stream, highest_block_pair};
/// # let _ = RootCircuit::build(|circuit| -> Result<(), Error> {
/// #     let (blocks, _) = circuit.add_input_zset::<Block>();
/// #     let highest = highest_block_pair(&blocks);
/// #     let (slopes, _) = circuit.add_input_zset::<BlockSlope>();
/// #     let _ = floor_height_stream(&highest, &slopes);
/// #     Ok(())
/// # });
/// ```
#[must_use]
pub fn floor_height_stream(
    highest_pair: &Stream<RootCircuit, OrdZSet<(HighestBlockAt, i64)>>,
    slopes: &Stream<RootCircuit, OrdZSet<BlockSlope>>,
) -> Stream<RootCircuit, OrdZSet<FloorHeightAt>> {
    highest_pair
        .map_index(|(hb, id)| (*id, (hb.x, hb.y, hb.z)))
        .outer_join(
            &slopes.map_index(|bs| (bs.block_id, (bs.grad_x, bs.grad_y))),
            |_, &(x, y, z), &(gx, gy)| {
                let base = f64::from(z) + BLOCK_TOP_OFFSET;
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
                    z: OrderedFloat(f64::from(z) + BLOCK_TOP_OFFSET),
                })
            },
            |_, _| None,
        )
        // Convert `Option<FloorHeightAt>` from the outer join, discarding
        // unmatched slope records.
        .flat_map(|fh| (*fh).into_iter())
}

#[cfg(test)]
mod tests;
