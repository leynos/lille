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
/// ```rust,no_run
/// use dbsp::RootCircuit;
/// use lille::components::Block;
/// use lille::dbsp_circuit::streams::floor::highest_block_pair;
///
/// let (mut circuit, blocks_in, mut highest_out) = RootCircuit::build(|circuit| {
///     let (blocks_stream, blocks_input) = circuit.add_input_zset::<Block>();
///     let highest = highest_block_pair(&blocks_stream).output();
///     Ok((blocks_input, highest))
/// })
/// .expect("failed to build circuit");
///
/// blocks_in.push(Block { id: 1, x: 0, y: 0, z: 3 }, 1);
/// blocks_in.push(Block { id: 2, x: 0, y: 0, z: 5 }, 1);
/// blocks_in.push(Block { id: 3, x: 1, y: 0, z: 2 }, 1);
///
/// circuit.step().expect("evaluation failed");
///
/// let maxima: Vec<_> = highest_out
///     .consolidate()
///     .iter()
///     .map(|(highest, (), _)| highest.clone())
///     .collect();
/// assert_eq!(maxima.len(), 2);
/// assert!(maxima.iter().any(|h| h.x == 0 && h.y == 0 && h.z.into_inner() == 5));
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
/// ```rust,no_run
/// use dbsp::RootCircuit;
/// use lille::components::{Block, BlockSlope};
/// use lille::dbsp_circuit::streams::floor::{floor_height_stream, highest_block_pair};
/// use ordered_float::OrderedFloat;
///
/// let (mut circuit, block_in, slope_in, mut floor_out) = RootCircuit::build(|circuit| {
///     let (blocks_stream, blocks_input) = circuit.add_input_zset::<Block>();
///     let (slopes_stream, slopes_input) = circuit.add_input_zset::<BlockSlope>();
///     let highest = highest_block_pair(&blocks_stream);
///     let floor = floor_height_stream(&highest, &slopes_stream).output();
///     Ok((blocks_input, slopes_input, floor))
/// })
/// .expect("failed to build circuit");
///
/// block_in.push(Block { id: 10, x: 0, y: 0, z: 4 }, 1);
/// block_in.push(Block { id: 11, x: 0, y: 0, z: 5 }, 1);
/// block_in.push(Block { id: 12, x: 1, y: 0, z: 3 }, 1);
///
/// slope_in.push(
///     BlockSlope {
///         block_id: 11,
///         grad_x: OrderedFloat(0.5),
///         grad_y: OrderedFloat(-0.25),
///     },
///     1,
/// );
///
/// circuit.step().expect("evaluation failed");
///
/// let heights: Vec<_> = floor_out
///     .consolidate()
///     .iter()
///     .map(|(height, (), _)| height.clone())
///     .collect();
/// assert_eq!(heights.len(), 2);
/// let origin_height = heights
///     .iter()
///     .find(|h| h.x == 0 && h.y == 0)
///     .expect("origin cell");
/// assert!(origin_height.z.into_inner() > 5.0, "slope raises the floor height");
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
