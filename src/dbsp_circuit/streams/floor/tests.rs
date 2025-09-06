//! Tests for floor streams aggregating block heights and slopes.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::step_named;
use crate::dbsp_circuit::streams::test_utils::{
    block, new_circuit, slope, 
};
use crate::dbsp_circuit::{FloorHeightAt, HighestBlockAt};
use rstest::rstest;

fn hb(x: i32, y: i32, z: i32) -> HighestBlockAt {
    HighestBlockAt { x, y, z }
}

fn fh(x: i32, y: i32, height: f64) -> FloorHeightAt {
    FloorHeightAt {
        x,
        y,
        z: height.into(),
    }
}

#[test]
fn test_highest_block_aggregation() {
    let mut circuit = new_circuit();

    circuit
        .block_in()
        .push(block(0.into(), (10, 20, 5).into()), 1);
    circuit
        .block_in()
        .push(block(1.into(), (10, 20, 8).into()), 1);
    circuit
        .block_in()
        .push(block(2.into(), (15, 25, 3).into()), 1);

    step_named(&mut circuit, "test_highest_block_aggregation");

    let mut vals: Vec<HighestBlockAt> = circuit
        .highest_block_out()
        .consolidate()
        .iter()
        .map(|(hb, _, _)| hb)
        .collect();
    vals.sort_by_key(|h| (h.x, h.y));
    assert!(vals
        .windows(2)
        .all(|w| w[0].x != w[1].x || w[0].y != w[1].y));
    assert_eq!(vals.len(), 2);
    assert!(vals.contains(&HighestBlockAt { x: 10, y: 20, z: 8 }));
    assert!(vals.contains(&HighestBlockAt { x: 15, y: 25, z: 3 }));
}

#[rstest]
#[case::empty(vec![], vec![])]
#[case::single(vec![block(1.into(), (0, 0, 2).into())], vec![hb(0, 0, 2)])]
#[case::duplicate_same_height(vec![block(1.into(), (1, 1, 5).into()), block(2.into(), (1, 1, 5).into())], vec![hb(1,1,5)])]
#[case::mixed(vec![block(1.into(), (0, 0, 3).into()), block(2.into(), (0, 0, 1).into()), block(3.into(), (0, 1, 4).into())], vec![hb(0,0,3), hb(0,1,4)])]
fn highest_block_cases(#[case] blocks: Vec<Block>, #[case] expected: Vec<HighestBlockAt>) {
    let mut circuit = new_circuit();
    for blk in blocks {
        circuit.block_in().push(blk, 1);
    }
    step_named(&mut circuit, "highest_block_cases");

    let mut vals: Vec<HighestBlockAt> = circuit
        .highest_block_out()
        .consolidate()
        .iter()
        .map(|(hb, _, _)| hb)
        .collect();
    vals.sort_by_key(|h| (h.x, h.y));

    let mut expected_sorted = expected;
    expected_sorted.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals, expected_sorted);
}

#[rstest]
#[case::block_only(vec![block(1.into(), (0, 0, 0).into())], vec![], vec![fh(0,0,1.0)])]
#[case::block_with_slope(vec![block(1.into(), (0, 0, 0).into())], vec![slope(1.into(), (1.0, 0.0).into())], vec![fh(0,0,1.5)])]
#[case::highest_block_wins(vec![block(1.into(), (0, 0, 0).into()), block(2.into(), (0, 0, 1).into())], vec![], vec![fh(0,0,2.0)])] // highest block wins
#[case::negative_slope(vec![block(1.into(), (0, 0, 0).into())], vec![slope(1.into(), (-1.0, 0.0).into())], vec![fh(0,0,0.5)])] // negative slope
#[case::zero_slope(vec![block(1.into(), (0, 0, 0).into())], vec![slope(1.into(), (0.0, 0.0).into())], vec![fh(0,0,1.0)])] // zero slope
#[case::negative_coords(vec![block(1.into(), (-1, -1, 0).into())], vec![slope(1.into(), (1.0, 1.0).into())], vec![fh(-1,-1,2.0)])] // negative coordinates
#[case::large_gradients(vec![block(1.into(), (0, 0, 0).into())], vec![slope(1.into(), (100.0, 100.0).into())], vec![fh(0,0,101.0)])] // large gradients
#[case::multiple_slopes(vec![block(1.into(), (0, 0, 0).into()), block(2.into(), (0, 0, 1).into())], vec![slope(1.into(), (1.0, 0.0).into()), slope(2.into(), (0.0, 1.0).into())], vec![fh(0,0,2.5)])] // multiple slopes, highest wins
fn floor_height_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] expected: Vec<FloorHeightAt>,
) {
    let mut circuit = new_circuit();
    for b in blocks {
        circuit.block_in().push(b, 1);
    }
    for s in slopes {
        circuit.block_slope_in().push(s, 1);
    }
    step_named(&mut circuit, "floor_height_cases");
    let mut vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|(fh, _, _)| fh)
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
        .push(block(1.into(), (0, 0, 0).into()), 1);
    circuit
        .block_slope_in()
        .push(slope(2.into(), (1.0, 0.0).into()), 1);

    step_named(&mut circuit, "unmatched_slope_is_ignored");

    let vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|(fh, _, _)| fh)
        .collect();

    assert_eq!(vals, vec![fh(0, 0, 1.0)]);
}

#[test]
fn slope_without_block_yields_no_height() {
    let mut circuit = new_circuit();
    circuit
        .block_slope_in()
        .push(slope(1.into(), (1.0, 0.0).into()), 1);

    step_named(&mut circuit, "slope_without_block_yields_no_height");

    let vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|(fh, _, _)| fh)
        .collect();

    assert!(vals.is_empty());
}
