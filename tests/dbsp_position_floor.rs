//! Unit tests for joining continuous positions with the floor grid.
//!
//! The helper functions build small DBSP circuits to ensure that
//! `position_floor_stream` associates entities with the correct
//! floor height across a range of scenarios.
use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::{Position, PositionFloor},
};
mod common;
use common::pos;
use rstest::rstest;

/// Creates a [`Block`] positioned at integer grid coordinates.
fn blk(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}

/// Constructs a [`BlockSlope`] for use in test cases.
fn slope(block_id: i64, gx: f64, gy: f64) -> BlockSlope {
    BlockSlope {
        block_id,
        grad_x: gx.into(),
        grad_y: gy.into(),
    }
}

/// Helper to create expected [`PositionFloor`] outputs.
fn pf(position: Position, z_floor: f64) -> PositionFloor {
    PositionFloor {
        position,
        z_floor: z_floor.into(),
    }
}

#[rstest]
#[case(
    vec![blk(1,0,0,0)],
    vec![],
    vec![pos(1,0.2,0.3,2.0)],
    vec![pf(pos(1,0.2,0.3,2.0),1.0)],
)]
#[case(
    vec![],
    vec![],
    vec![pos(1,0.0,0.0,0.5)],
    vec![],
)]
#[case(
    vec![blk(1,-1,-1,0)],
    vec![slope(1,1.0,0.0)],
    vec![pos(2,-0.8,-0.2,3.0)],
    vec![pf(pos(2,-0.8,-0.2,3.0),1.5)],
)]
/// Asserts that the position-floor join yields the expected records.
fn position_floor_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] positions: Vec<Position>,
    #[case] expected: Vec<PositionFloor>,
) {
    let mut circuit = common::new_circuit();
    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    for s in &slopes {
        circuit.block_slope_in().push(s.clone(), 1);
    }
    for p in &positions {
        circuit.position_in().push(p.clone(), 1);
    }
    circuit.step().expect("step");
    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, _, _)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);
    let mut exp = expected;
    exp.sort_by_key(|pf| pf.position.entity);
    assert_eq!(vals, exp);
}

/// Ensures multiple entities in the same grid cell each receive a
/// corresponding [`PositionFloor`] record.
#[test]
fn multiple_positions_same_grid_cell() {
    let mut circuit = common::new_circuit();
    circuit.block_in().push(blk(1, 0, 0, 0), 1);
    circuit.position_in().push(pos(1, 0.1, 0.1, 2.0), 1);
    circuit.position_in().push(pos(2, 0.8, 0.4, 3.0), 1);
    circuit.step().expect("step");

    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, _, _)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);

    let mut exp = vec![
        pf(pos(1, 0.1, 0.1, 2.0), 1.0),
        pf(pos(2, 0.8, 0.4, 3.0), 1.0),
    ];
    exp.sort_by_key(|pf| pf.position.entity);

    assert_eq!(vals, exp);
}
