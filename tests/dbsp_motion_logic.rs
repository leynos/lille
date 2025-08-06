//! Unit tests for DBSP motion logic.
//!
//! These tests verify the standing vs unsupported filters and resulting
//! position and velocity calculations.

use approx::assert_relative_eq;
use lille::components::Block;
use lille::dbsp_circuit::{NewPosition, NewVelocity, Position, Velocity};
use lille::GRAVITY_PULL;
use rstest::rstest;

mod common;
use common::new_circuit;

fn vel(entity: i64, vx: f64, vy: f64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: vx.into(),
        vy: vy.into(),
        vz: vz.into(),
    }
}

fn block(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}

#[rstest]
#[case::standing_moves(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 1.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0), block(2, 1, 0, 1)],
    Position { entity: 1, x: 1.0.into(), y: 0.0.into(), z: 2.0.into() },
    vel(1, 1.0, 0.0, 0.0),
)]
#[case::unsupported_falls(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() },
    vel(1, 0.0, 0.0, GRAVITY_PULL),
)]
#[case::boundary_snaps_to_floor(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 0.0, 0.0, 0.0),
)]
fn motion_cases(
    #[case] position: Position,
    #[case] velocity: Velocity,
    #[case] blocks: Vec<Block>,
    #[case] expected_pos: NewPosition,
    #[case] expected_vel: NewVelocity,
) {
    let mut circuit = new_circuit();

    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    circuit.position_in().push(position, 1);
    circuit.velocity_in().push(velocity, 1);

    circuit.step().expect("circuit step failed");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|(p, _, _)| p.clone())
        .collect();
    assert_eq!(pos_out.len(), 1);
    assert_eq!(pos_out[0].entity, expected_pos.entity);
    assert_relative_eq!(pos_out[0].x.into_inner(), expected_pos.x.into_inner());
    assert_relative_eq!(pos_out[0].y.into_inner(), expected_pos.y.into_inner());
    assert_relative_eq!(pos_out[0].z.into_inner(), expected_pos.z.into_inner());

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|(v, _, _)| v.clone())
        .collect();
    assert_eq!(vel_out.len(), 1);
    assert_eq!(vel_out[0].entity, expected_vel.entity);
    assert_relative_eq!(vel_out[0].vx.into_inner(), expected_vel.vx.into_inner());
    assert_relative_eq!(vel_out[0].vy.into_inner(), expected_vel.vy.into_inner());
    assert_relative_eq!(vel_out[0].vz.into_inner(), expected_vel.vz.into_inner());
}
