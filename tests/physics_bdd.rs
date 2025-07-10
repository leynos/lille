//! Physics behaviour-driven development tests using the DBSP circuit.
//!
//! This module tests physics rules such as gravity effects on entity positions
//! through the declarative dataflow circuit.

use approx::assert_relative_eq;
use lille::dbsp_circuit::{NewPosition, NewVelocity, Position, Velocity};
use rstest::rstest;
mod common;

/// Tests that an entity's position and velocity are updated correctly under gravity in the physics circuit.
///
/// This test initialises an entity at a given position with zero velocity, steps the DBSP circuit,
/// and asserts that the entity's new position and velocity reflect the effect of gravity.
///
/// # Examples
///
/// ```no_run
/// entity_falls_due_to_gravity();
/// ```
#[test]
fn entity_falls_due_to_gravity() {
    let mut circuit = common::new_circuit();

    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(
        Velocity {
            entity: 1,
            vx: 0.0.into(),
            vy: 0.0.into(),
            vz: 0.0.into(),
        },
        1,
    );
    circuit.step().expect("Failed to step DBSP circuit");

    let output = circuit.new_position_out().consolidate();
    let results: Vec<NewPosition> = output.iter().map(|(p, _, _)| p.clone()).collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].entity, 1);
    assert_relative_eq!(results[0].z.into_inner(), 1.0 + lille::GRAVITY_PULL);

    let vout = circuit.new_velocity_out().consolidate();
    let vresults: Vec<NewVelocity> = vout.iter().map(|(v, _, _)| v.clone()).collect();
    assert_eq!(vresults.len(), 1);
    assert_eq!(vresults[0].entity, 1);
    assert_relative_eq!(vresults[0].vz.into_inner(), lille::GRAVITY_PULL);
}

fn pos(entity: i64, x: f64, y: f64, z: f64) -> Position {
    Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

fn vel(entity: i64, vx: f64, vy: f64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: vx.into(),
        vy: vy.into(),
        vz: vz.into(),
    }
}

#[rstest]
#[case::non_zero_initial_velocity(
    vec![pos(1, 0.0, 0.0, 10.0)],
    vec![vel(1, 1.0, 0.0, 2.0)],
    vec![pos(1, 1.0, 0.0, 10.0 + 2.0 + lille::GRAVITY_PULL)],
    vec![vel(1, 1.0, 0.0, 2.0 + lille::GRAVITY_PULL)],
)]
#[case::multiple_entities(
    vec![pos(1, 0.0, 0.0, 0.0), pos(2, 1.0, 1.0, 1.0)],
    vec![vel(1, 0.0, 0.0, 0.0), vel(2, 0.5, -0.5, -0.5)],
    vec![
        pos(1, 0.0, 0.0, 0.0 + 0.0 + lille::GRAVITY_PULL),
        pos(2, 1.5, 0.5, 1.0 - 0.5 + lille::GRAVITY_PULL),
    ],
    vec![
        vel(1, 0.0, 0.0, 0.0 + lille::GRAVITY_PULL),
        vel(2, 0.5, -0.5, -0.5 + lille::GRAVITY_PULL),
    ],
)]
#[case::position_without_velocity(
    vec![pos(1, 0.0, 0.0, 5.0)],
    vec![],
    vec![],
    vec![],
)]
#[case::velocity_without_position(
    vec![],
    vec![vel(3, 1.0, 2.0, 3.0)],
    vec![],
    vec![vel(3, 1.0, 2.0, 3.0 + lille::GRAVITY_PULL)],
)]
fn gravity_cases(
    #[case] positions: Vec<Position>,
    #[case] velocities: Vec<Velocity>,
    #[case] expected_positions: Vec<NewPosition>,
    #[case] expected_velocities: Vec<NewVelocity>,
) {
    let mut circuit = common::new_circuit();

    for p in &positions {
        circuit.position_in().push(p.clone(), 1);
    }
    for v in &velocities {
        circuit.velocity_in().push(v.clone(), 1);
    }

    circuit.step().expect("Failed to step DBSP circuit");

    let mut pos_results: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|(p, _, _)| p.clone())
        .collect();
    pos_results.sort_by_key(|p| p.entity);
    let mut expected_pos = expected_positions;
    expected_pos.sort_by_key(|p| p.entity);
    assert_eq!(pos_results.len(), expected_pos.len());
    for (res, exp) in pos_results.iter().zip(expected_pos.iter()) {
        assert_eq!(res.entity, exp.entity);
        assert_relative_eq!(res.x.into_inner(), exp.x.into_inner());
        assert_relative_eq!(res.y.into_inner(), exp.y.into_inner());
        assert_relative_eq!(res.z.into_inner(), exp.z.into_inner());
    }

    let mut vel_results: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|(v, _, _)| v.clone())
        .collect();
    vel_results.sort_by_key(|v| v.entity);
    let mut expected_vel = expected_velocities;
    expected_vel.sort_by_key(|v| v.entity);
    assert_eq!(vel_results.len(), expected_vel.len());
    for (res, exp) in vel_results.iter().zip(expected_vel.iter()) {
        assert_eq!(res.entity, exp.entity);
        assert_relative_eq!(res.vx.into_inner(), exp.vx.into_inner());
        assert_relative_eq!(res.vy.into_inner(), exp.vy.into_inner());
        assert_relative_eq!(res.vz.into_inner(), exp.vz.into_inner());
    }
}
