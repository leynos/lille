use dbsp::{
    operator::Max, typed_batch::OrdZSet, CircuitHandle, OutputHandle, RootCircuit, ZSetHandle,
};

use crate::components::Block;
use crate::GRAVITY_PULL;

#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub entity: i64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewPosition {
    pub entity: i64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HighestBlockAt {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub struct DbspCircuit {
    circuit: CircuitHandle,
    pub position_in: ZSetHandle<Position>,
    pub block_in: ZSetHandle<Block>,
    pub new_position_out: OutputHandle<OrdZSet<NewPosition>>,
    pub highest_block_out: OutputHandle<OrdZSet<HighestBlockAt>>,
}

impl DbspCircuit {
    pub fn new() -> Result<Self, dbsp::Error> {
        let (circuit, (position_in, block_in, new_position_out, highest_block_out)) =
            RootCircuit::build(|circuit| {
                let (positions, position_in) = circuit.add_input_zset::<Position>();
                let (blocks, block_in) = circuit.add_input_zset::<Block>();

                let highest = blocks
                    .map_index(|b| ((b.x, b.y), b.z))
                    .aggregate(Max)
                    .map(|((x, y), z)| HighestBlockAt { x, y, z });

                let new_pos = positions.map(|p| NewPosition {
                    entity: p.entity,
                    x: p.x,
                    y: p.y,
                    z: p.z + GRAVITY_PULL,
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
}
