//! DBSP-based world inference engine.
//!
//! This module defines [`DbspCircuit`], the authoritative dataflow program for
//! Lille's game world. Callers feed [`Position`] and [`Block`] records into the
//! circuit, call [`DbspCircuit::step`], then read [`NewPosition`] and
//! [`HighestBlockAt`] outputs. Input collections persist across steps—invoke
//! [`DbspCircuit::clear_inputs`] after each frame to prevent stale data from
//! affecting subsequent computations.

use dbsp::{
    operator::Max, typed_batch::OrdZSet, CircuitHandle, OutputHandle, RootCircuit, ZSetHandle,
};
use ordered_float::OrderedFloat;
use size_of::SizeOf;

use crate::components::Block;
use crate::GRAVITY_PULL;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct Position {
    pub entity: i64,
    pub x: OrderedFloat<f64>,
    pub y: OrderedFloat<f64>,
    pub z: OrderedFloat<f64>,
}

pub type NewPosition = Position;

#[derive(
    Archive,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct HighestBlockAt {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub struct DbspCircuit {
    circuit: CircuitHandle,
    position_in: ZSetHandle<Position>,
    block_in: ZSetHandle<Block>,
    new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
}

impl DbspCircuit {
    pub fn new() -> Result<Self, dbsp::Error> {
        let (circuit, (position_in, block_in, new_position_out, highest_block_out)) =
            RootCircuit::build(|circuit| {
                let (positions, position_in) = circuit.add_input_zset::<Position>();
                let (blocks, block_in) = circuit.add_input_zset::<Block>();

                let highest =
                    blocks
                        .map_index(|b| ((b.x, b.y), b.z))
                        .aggregate(Max)
                        .map(|((x, y), z)| HighestBlockAt {
                            x: *x,
                            y: *y,
                            z: *z,
                        });

                let new_pos = positions.map(|p| Position {
                    entity: p.entity,
                    x: p.x,
                    y: p.y,
                    z: OrderedFloat(p.z.into_inner() + GRAVITY_PULL),
                });

                Ok((position_in, block_in, new_pos.output(), highest.output()))
            })?;

        Ok(Self {
            circuit,
            position_in,
            block_in,
            new_position_out,
            highest_block_out,
        })
    }

    pub fn step(&mut self) -> Result<(), dbsp::Error> {
        self.circuit.step()
    }

    /// Returns the handle used to feed position records into the circuit.
    pub fn position_in(&self) -> &ZSetHandle<Position> {
        &self.position_in
    }

    /// Returns the handle used to feed block records into the circuit.
    pub fn block_in(&self) -> &ZSetHandle<Block> {
        &self.block_in
    }

    /// Returns the output handle containing newly computed positions.
    pub fn new_position_out(&self) -> &OutputHandle<OrdZSet<NewPosition>> {
        &self.new_position_out
    }

    /// Returns the output handle of the highest block aggregation.
    pub fn highest_block_out(&self) -> &OutputHandle<OrdZSet<HighestBlockAt>> {
        &self.highest_block_out
    }

    /// Clears all input collections.
    ///
    /// Input ZSets accumulate records across [`DbspCircuit::step`] calls.
    /// Call this method after each frame, once outputs have been processed,
    /// to prevent stale data from affecting subsequent computations.
    pub fn clear_inputs(&mut self) {
        self.position_in.clear_input();
        self.block_in.clear_input();
    }
}
