//! Tests for joining positions with floor heights.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::step_named;
use crate::dbsp_circuit::streams::test_utils::{block, new_circuit, pos, slope};
use crate::dbsp_circuit::{Position, PositionFloor};
use rstest::rstest;

fn pf(position: Position, z_floor: f64) -> PositionFloor {
    PositionFloor {
        position,
        z_floor: z_floor.into(),
    }
}

#[rstest]
#[case(
    vec![block(1, (0, 0, 0))],
    vec![],
    vec![pos(1, (0.2, 0.3, 2.0))],
    vec![pf(pos(1, (0.2, 0.3, 2.0)),1.0)],
)]
#[case(
    vec![],
    vec![],
    vec![pos(1, (0.0, 0.0, 0.5))],
    vec![],
)]
#[case(
    vec![block(1, (-1, -1, 0))],
    vec![slope(1, (1.0, 0.0))],
    vec![pos(2, (-0.8, -0.2, 3.0))],
    vec![pf(pos(2, (-0.8, -0.2, 3.0)),1.5)],
)]
fn position_floor_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] positions: Vec<Position>,
    #[case] expected: Vec<PositionFloor>,
) {
    let mut circuit = new_circuit();
    for b in blocks {
        circuit.block_in().push(b, 1);
    }
    for s in slopes {
        circuit.block_slope_in().push(s, 1);
    }
    for p in positions {
        circuit.position_in().push(p, 1);
    }
    step_named(&mut circuit, "position_floor_cases");
    // `consolidate()` yields a `TypedBatch` without `IntoIterator`; clone values for comparison.
    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, (), _timestamp)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);
    let mut exp = expected;
    exp.sort_by_key(|pf| pf.position.entity);
    assert_eq!(vals, exp);
}

#[test]
fn multiple_positions_same_grid_cell() {
    let mut circuit = new_circuit();
    circuit.block_in().push(block(1, (0, 0, 0)), 1);
    circuit.position_in().push(pos(1, (0.1, 0.1, 2.0)), 1);
    circuit.position_in().push(pos(2, (0.8, 0.4, 3.0)), 1);
    step_named(&mut circuit, "multiple_positions_same_grid_cell");
    // `consolidate()` yields a `TypedBatch` without `IntoIterator`; clone values for comparison.
    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, (), _timestamp)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);
    let mut exp = vec![
        pf(pos(1, (0.1, 0.1, 2.0)), 1.0),
        pf(pos(2, (0.8, 0.4, 3.0)), 1.0),
    ];
    exp.sort_by_key(|pf| pf.position.entity);
    assert_eq!(vals, exp);
}
