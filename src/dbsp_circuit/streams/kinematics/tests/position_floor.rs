//! Tests for joining positions with floor heights.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::streams::test_utils::{
    block, new_circuit, pos, slope, step, BlockCoords, BlockId, Coords3D, EntityId, Gradient,
};
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
    vec![block(BlockId::new(1), BlockCoords::new(0, 0, 0))],
    vec![],
    vec![pos(EntityId::new(1), Coords3D::new(0.2, 0.3, 2.0))],
    vec![pf(pos(EntityId::new(1), Coords3D::new(0.2, 0.3, 2.0)),1.0)],
)]
#[case(
    vec![],
    vec![],
    vec![pos(EntityId::new(1), Coords3D::new(0.0, 0.0, 0.5))],
    vec![],
)]
#[case(
    vec![block(BlockId::new(1), BlockCoords::new(-1, -1, 0))],
    vec![slope(BlockId::new(1), Gradient::new(1.0, 0.0))],
    vec![pos(EntityId::new(2), Coords3D::new(-0.8, -0.2, 3.0))],
    vec![pf(pos(EntityId::new(2), Coords3D::new(-0.8, -0.2, 3.0)),1.5)],
)]
fn position_floor_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] positions: Vec<Position>,
    #[case] expected: Vec<PositionFloor>,
) {
    let mut circuit = new_circuit();
    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    for s in &slopes {
        circuit.block_slope_in().push(s.clone(), 1);
    }
    for p in positions {
        circuit.position_in().push(p, 1);
    }
    step(&mut circuit);
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

#[test]
fn multiple_positions_same_grid_cell() {
    let mut circuit = new_circuit();
    circuit
        .block_in()
        .push(block(BlockId::new(1), BlockCoords::new(0, 0, 0)), 1);
    circuit
        .position_in()
        .push(pos(EntityId::new(1), Coords3D::new(0.1, 0.1, 2.0)), 1);
    circuit
        .position_in()
        .push(pos(EntityId::new(2), Coords3D::new(0.8, 0.4, 3.0)), 1);
    step(&mut circuit);
    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, _, _)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);
    let mut exp = vec![
        pf(pos(EntityId::new(1), Coords3D::new(0.1, 0.1, 2.0)), 1.0),
        pf(pos(EntityId::new(2), Coords3D::new(0.8, 0.4, 3.0)), 1.0),
    ];
    exp.sort_by_key(|pf| pf.position.entity);
    assert_eq!(vals, exp);
}
