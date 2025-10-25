//! Behavioural tests validating slope-aware floor height calculations.
//!
//! These rspec-style tests build a single-threaded DBSP circuit and verify
//! that block slopes modify the computed ground height as expected.
use anyhow::{ensure, Context, Result};
use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::FloorHeightAt,
    DbspCircuit,
};
use test_utils::step;

struct Env {
    circuit: DbspCircuit,
}

impl Env {
    fn new() -> Result<Self> {
        let circuit = DbspCircuit::new().context("failed to create DbspCircuit")?;
        Ok(Self { circuit })
    }

    fn push(&mut self, block: Block, maybe_slope: Option<BlockSlope>) {
        self.circuit.block_in().push(block, 1);
        if let Some(slope) = maybe_slope {
            self.circuit.block_slope_in().push(slope, 1);
        }
    }

    fn step(&mut self) {
        step(&mut self.circuit);
    }

    #[expect(
        clippy::ignored_unit_patterns,
        reason = "DBSP batches include weight/time metadata that tests intentionally skip"
    )]
    fn output(&mut self) -> Vec<FloorHeightAt> {
        let vals: Vec<_> = self
            .circuit
            .floor_height_out()
            .consolidate()
            .iter()
            .map(|(fh, (), _timestamp)| *fh)
            .collect();
        self.circuit.clear_inputs();
        vals
    }
}

#[test]
fn slope_block_outputs_height() -> Result<()> {
    let mut env = Env::new()?;
    env.push(
        Block {
            id: 1,
            x: 0,
            y: 0,
            z: 0,
        },
        Some(BlockSlope {
            block_id: 1,
            grad_x: 1.0.into(),
            grad_y: 0.0.into(),
        }),
    );
    env.step();
    let out = env.output();
    let expected = vec![FloorHeightAt {
        x: 0,
        y: 0,
        z: 1.5.into(),
    }];
    ensure!(
        out == expected,
        "unexpected floor heights: expected {expected:?}, observed {out:?}"
    );
    Ok(())
}
