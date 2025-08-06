//! Tests for the highest block aggregation functionality in the DBSP circuit.
//!
//! This module validates that the circuit correctly computes the highest block
//! at each `(x, y)` coordinate pair from multiple input blocks.

use lille::{components::Block, dbsp_circuit::HighestBlockAt};
mod common;
use common::block;
use rstest::rstest;

fn hb(x: i32, y: i32, z: i32) -> HighestBlockAt {
    HighestBlockAt { x, y, z }
}

#[test]
fn test_highest_block_aggregation() {
    let mut circuit = common::new_circuit();

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

#[rstest]
#[case::empty(vec![], vec![])]
#[case::single(vec![block(1, 0, 0, 2)], vec![hb(0, 0, 2)])]
#[case::duplicate_same_height(vec![block(1,1,1,5), block(2,1,1,5)], vec![hb(1,1,5)])]
#[case::mixed(vec![block(1,0,0,3), block(2,0,0,1), block(3,0,1,4)], vec![hb(0,0,3), hb(0,1,4)])]
fn highest_block_cases(#[case] blocks: Vec<Block>, #[case] expected: Vec<HighestBlockAt>) {
    let mut circuit = common::new_circuit();
    for blk in blocks {
        circuit.block_in().push(blk, 1);
    }
    circuit.step().expect("Failed to step DBSP circuit");

    let mut vals: Vec<HighestBlockAt> = circuit
        .highest_block_out()
        .consolidate()
        .iter()
        .map(|(hb, _, _)| hb.clone())
        .collect();
    vals.sort_by_key(|h| (h.x, h.y));

    let mut expected_sorted = expected;
    expected_sorted.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals, expected_sorted);
}
