//! Unit tests for DBSP motion logic.
//!
//! These tests verify the standing vs unsupported filters and resulting
//! position and velocity calculations.

use approx::assert_relative_eq;
use lille::components::Block;
use lille::dbsp_circuit::{Force, NewPosition, NewVelocity, Position, Velocity};
use lille::GRAVITY_PULL;
use rstest::rstest;
use test_utils::{block, force, force_with_mass, new_circuit, vel};

#[rstest]
#[case::standing_moves(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 1.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0), block(2, 1, 0, 1)],
    None,
    Some(Position { entity: 1, x: 1.0.into(), y: 0.0.into(), z: 2.0.into() }),
    Some(vel(1, 1.0, 0.0, 0.0)),
)]
#[case::unsupported_falls(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    None,
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() }),
    Some(vel(1, 0.0, 0.0, GRAVITY_PULL)),
)]
#[case::boundary_snaps_to_floor(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    None,
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, 0.0, 0.0, 0.0)),
)]
#[case::force_accelerates(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0), block(2, 1, 0, 1)],
    Some(force_with_mass(1, (5.0, 0.0, 0.0), 5.0)),
    Some(Position { entity: 1, x: 1.0.into(), y: 0.0.into(), z: 2.0.into() }),
    Some(vel(1, 1.0, 0.0, 0.0)),
)]
#[case::invalid_mass_ignores_force(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    Some(force_with_mass(1, (0.0, 0.0, 10.0), 0.0)),
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() }),
    Some(vel(1, 0.0, 0.0, GRAVITY_PULL)),
)]
#[case::force_with_default_mass(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    Some(force(1, (lille::DEFAULT_MASS, 0.0, 0.0))),
    None,
    None,
)]
fn motion_cases(
    #[case] position: Position,
    #[case] velocity: Velocity,
    #[case] blocks: Vec<Block>,
    #[case] force_rec: Option<Force>,
    #[case] expected_pos: Option<NewPosition>,
    #[case] expected_vel: Option<NewVelocity>,
) {
    let mut circuit = new_circuit();

    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    circuit.position_in().push(position, 1);
    circuit.velocity_in().push(velocity, 1);
    if let Some(f) = force_rec {
        circuit.force_in().push(f, 1);
    }

    circuit.step().expect("circuit step failed");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    match expected_pos {
        Some(expected) => {
            assert_eq!(pos_out.len(), 1);
            assert_eq!(pos_out[0].entity, expected.entity);
            assert_relative_eq!(pos_out[0].x.into_inner(), expected.x.into_inner());
            assert_relative_eq!(pos_out[0].y.into_inner(), expected.y.into_inner());
            assert_relative_eq!(pos_out[0].z.into_inner(), expected.z.into_inner());
        }
        None => assert!(pos_out.is_empty()),
    }

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    match expected_vel {
        Some(expected) => {
            assert_eq!(vel_out.len(), 1);
            assert_eq!(vel_out[0].entity, expected.entity);
            assert_relative_eq!(vel_out[0].vx.into_inner(), expected.vx.into_inner());
            assert_relative_eq!(vel_out[0].vy.into_inner(), expected.vy.into_inner());
            assert_relative_eq!(vel_out[0].vz.into_inner(), expected.vz.into_inner());
        }
        None => assert!(vel_out.is_empty()),
    }
}
