//! Tests for the highest block aggregation functionality in the DBSP circuit.
//!
//! This module validates that the circuit correctly computes the highest block
//! at each `(x, y)` coordinate pair from multiple input blocks.

use lille::{
    components::Block,
    dbsp_circuit::{DbspCircuit, HighestBlockAt},
};

#[test]
fn test_highest_block_aggregation() {
    let mut circuit = DbspCircuit::new().expect("Failed to create DBSP circuit");

    circuit.block_in().push(
        Block {
            id: 0,
            x: 10,
            y: 20,
            z: 5,
        },
        1,
    );
    circuit.block_in().push(
        Block {
            id: 1,
            x: 10,
            y: 20,
            z: 8,
        },
        1,
    );
    circuit.block_in().push(
        Block {
            id: 2,
            x: 15,
            y: 25,
            z: 3,
        },
        1,
    );

    circuit.step().expect("Failed to step DBSP circuit");

    let output = circuit.highest_block_out().consolidate();
    let mut vals: Vec<HighestBlockAt> = output.iter().map(|(hb, _, _)| hb.clone()).collect();
    vals.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals.len(), 2);
    assert!(vals.contains(&HighestBlockAt { x: 10, y: 20, z: 8 }));
    assert!(vals.contains(&HighestBlockAt { x: 15, y: 25, z: 3 }));
}
