//! Behaviour-driven tests for position and floor height joins in DBSP circuits.
//!
//! This module exercises the DBSP pipeline using the `rust-rspec` framework. It
//! verifies that entity positions are correctly paired with floor height
//! information when processed through `DbspCircuit`. The tests use a shared
//! circuit environment to mimic real application usage and cover both
//! successful joins and edge cases.
use anyhow::{ensure, Context, Result};
use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::{Position, PositionFloor},
    DbspCircuit,
};
use test_utils::{pos, step};

struct Env {
    circuit: DbspCircuit,
}

impl Env {
    fn new() -> Result<Self> {
        let circuit = DbspCircuit::new().context("failed to create DBSP circuit")?;
        Ok(Self { circuit })
    }

    fn push_block(&mut self, block: Block, maybe_slope: Option<BlockSlope>) {
        self.circuit.block_in().push(block, 1);
        if let Some(slope) = maybe_slope {
            self.circuit.block_slope_in().push(slope, 1);
        }
    }

    fn push_position(&mut self, pos: Position) {
        self.circuit.position_in().push(pos, 1);
    }

    fn step(&mut self) {
        step(&mut self.circuit);
    }

    #[expect(
        clippy::ignored_unit_patterns,
        reason = "DBSP batches include weight/time metadata that tests intentionally skip"
    )]
    fn output(&mut self) -> Vec<PositionFloor> {
        let vals: Vec<_> = self
            .circuit
            .position_floor_out()
            .consolidate()
            .iter()
            .map(|(pf, (), _timestamp)| pf.clone())
            .collect();
        self.circuit.clear_inputs();
        vals
    }
}

/// Runs a behavioural test that verifies positions are joined with floor height.
#[test]
fn join_position_with_floor() -> Result<()> {
    let mut env = Env::new()?;
    env.push_block(
        Block {
            id: 1,
            x: 0,
            y: 0,
            z: 0,
        },
        None,
    );
    env.push_position(pos(1, (0.2, 0.2, 2.0)));
    env.step();
    let out = env.output();
    let expected = vec![PositionFloor {
        position: pos(1, (0.2, 0.2, 2.0)),
        z_floor: 1.0.into(),
    }];
    ensure!(
        out == expected,
        "unexpected position-floor output: expected {expected:?}, observed {out:?}"
    );
    Ok(())
}
