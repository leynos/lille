//! Tests for floor streams aggregating block heights and slopes.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::streams::test_utils::{
    block, new_circuit, slope, BlockCoords, BlockId, Gradient,
};
use crate::dbsp_circuit::{FloorHeightAt, HighestBlockAt};
use rstest::rstest;

fn hb(x: i32, y: i32, z: i32) -> HighestBlockAt {
    HighestBlockAt { x, y, z }
}

fn fh(x: i32, y: i32, z: f64) -> FloorHeightAt {
    FloorHeightAt { x, y, z: z.into() }
}

#[test]
fn test_highest_block_aggregation() {
    let mut circuit = new_circuit();

    circuit
        .block_in()
        .push(block(BlockId::new(0), BlockCoords::new(10, 20, 5)), 1);
    circuit
        .block_in()
        .push(block(BlockId::new(1), BlockCoords::new(10, 20, 8)), 1);
    circuit
        .block_in()
        .push(block(BlockId::new(2), BlockCoords::new(15, 25, 3)), 1);

    circuit.step().expect("failed to step DBSP circuit");

    let output = circuit.highest_block_out().consolidate();
    let mut vals: Vec<HighestBlockAt> = output.iter().map(|t| t.0).collect();
    vals.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals.len(), 2);
    assert!(vals.contains(&HighestBlockAt { x: 10, y: 20, z: 8 }));
    assert!(vals.contains(&HighestBlockAt { x: 15, y: 25, z: 3 }));
}

#[rstest]
#[case::empty(vec![], vec![])]
#[case::single(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 2))], vec![hb(0, 0, 2)])]
#[case::duplicate_same_height(vec![block(BlockId::new(1), BlockCoords::new(1, 1, 5)), block(BlockId::new(2), BlockCoords::new(1, 1, 5))], vec![hb(1,1,5)])]
#[case::mixed(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 3)), block(BlockId::new(2), BlockCoords::new(0, 0, 1)), block(BlockId::new(3), BlockCoords::new(0, 1, 4))], vec![hb(0,0,3), hb(0,1,4)])]
fn highest_block_cases(#[case] blocks: Vec<Block>, #[case] expected: Vec<HighestBlockAt>) {
    let mut circuit = new_circuit();
    for blk in blocks {
        circuit.block_in().push(blk, 1);
    }
    circuit.step().expect("failed to step DBSP circuit");

    let mut vals: Vec<HighestBlockAt> = circuit
        .highest_block_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    vals.sort_by_key(|h| (h.x, h.y));

    let mut expected_sorted = expected;
    expected_sorted.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals, expected_sorted);
}

#[rstest]
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0))], vec![], vec![fh(0,0,1.0)])]
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0))], vec![slope(BlockId::new(1), Gradient::new(1.0, 0.0))], vec![fh(0,0,1.5)])]
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0)), block(BlockId::new(2), BlockCoords::new(0, 0, 1))], vec![], vec![fh(0,0,2.0)])] // highest block wins
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0))], vec![slope(BlockId::new(1), Gradient::new(-1.0, 0.0))], vec![fh(0,0,0.5)])] // negative slope
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0))], vec![slope(BlockId::new(1), Gradient::new(0.0, 0.0))], vec![fh(0,0,1.0)])] // zero slope
#[case(vec![block(BlockId::new(1), BlockCoords::new(-1, -1, 0))], vec![slope(BlockId::new(1), Gradient::new(1.0, 1.0))], vec![fh(-1,-1,2.0)])] // negative coordinates
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0))], vec![slope(BlockId::new(1), Gradient::new(100.0, 100.0))], vec![fh(0,0,101.0)])] // large gradients
#[case(vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0)), block(BlockId::new(2), BlockCoords::new(0, 0, 1))], vec![slope(BlockId::new(1), Gradient::new(1.0, 0.0)), slope(BlockId::new(2), Gradient::new(0.0, 1.0))], vec![fh(0,0,2.5)])] // multiple slopes, highest wins
fn floor_height_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] expected: Vec<FloorHeightAt>,
) {
    let mut circuit = new_circuit();
    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    for s in &slopes {
        circuit.block_slope_in().push(s.clone(), 1);
    }
    circuit.step().expect("step");
    let mut vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    vals.sort_by_key(|h| (h.x, h.y));
    let mut exp = expected;
    exp.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals, exp);
}

#[test]
fn unmatched_slope_is_ignored() {
    let mut circuit = new_circuit();
    circuit
        .block_in()
        .push(block(BlockId::new(1), BlockCoords::new(0, 0, 0)), 1);
    circuit
        .block_slope_in()
        .push(slope(BlockId::new(2), Gradient::new(1.0, 0.0)), 1);

    circuit.step().expect("step");

    let vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();

    assert_eq!(vals, vec![fh(0, 0, 1.0)]);
}

#[test]
fn slope_without_block_yields_no_height() {
    let mut circuit = new_circuit();
    circuit
        .block_slope_in()
        .push(slope(BlockId::new(1), Gradient::new(1.0, 0.0)), 1);

    circuit.step().expect("step");

    let vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();

    assert!(vals.is_empty());
}
