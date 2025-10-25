//! Tests for motion integration and ground interaction.

use crate::components::Block;
use crate::dbsp_circuit::step_named;
use crate::dbsp_circuit::streams::test_utils::{block, force, force_with_mass, new_circuit, vel};
use crate::dbsp_circuit::{Force, NewPosition, NewVelocity, Position, Velocity};
use crate::{apply_ground_friction, GRAVITY_PULL, TERMINAL_VELOCITY};
use approx::assert_relative_eq;
use rstest::rstest;

#[rstest]
#[case::standing_moves(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, (1.0, 0.0, 0.0)),
    vec![block(1, (0, 0, 0)), block(2, (1, 0, 1))],
    None,
    Some(Position { entity: 1, x: apply_ground_friction(1.0).into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, (apply_ground_friction(1.0), 0.0, 0.0))),
)]
#[case::unsupported_falls(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, (0.0, 0.0, 0.0)),
    vec![block(1, (0, 0, 0))],
    None,
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() }),
    Some(vel(1, (0.0, 0.0, GRAVITY_PULL))),
)]
#[case::boundary_snaps_to_floor(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() },
    vel(1, (0.0, 0.0, 0.0)),
    vec![block(1, (0, 0, 0))],
    None,
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, (0.0, 0.0, 0.0))),
)]
#[case::force_accelerates(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, (0.0, 0.0, 0.0)),
    vec![block(1, (0, 0, 0)), block(2, (1, 0, 1))],
    Some(force_with_mass(1, (5.0, 0.0, 0.0), 5.0)),
    Some(Position { entity: 1, x: apply_ground_friction(1.0).into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, (apply_ground_friction(1.0), 0.0, 0.0))),
)]
#[case::invalid_mass_ignores_force(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, (0.0, 0.0, 0.0)),
    vec![block(1, (0, 0, 0))],
    Some(force_with_mass(1, (0.0, 0.0, 10.0), 0.0)),
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() }),
    Some(vel(1, (0.0, 0.0, GRAVITY_PULL))),
)]
#[case::force_with_default_mass(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, (0.0, 0.0, 0.0)),
    vec![block(1, (0, 0, 0))],
    Some(force(1, (1.0, 0.0, 0.0))),
    Some(Position { entity: 1, x: apply_ground_friction(1.0 / crate::DEFAULT_MASS).into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, (apply_ground_friction(1.0 / crate::DEFAULT_MASS), 0.0, 0.0))),
)]
fn motion_cases(
    #[case] position: Position,
    #[case] velocity: Velocity,
    #[case] blocks: Vec<Block>,
    #[case] force_rec: Option<Force>,
    #[case] expected_pos: Option<NewPosition>,
    #[case] expected_vel: Option<NewVelocity>,
) {
    // DEFAULT_MASS is validated in `default_mass_is_positive`.
    let mut circuit = new_circuit();

    for b in blocks {
        circuit.block_in().push(b, 1);
    }
    circuit.position_in().push(position, 1);
    circuit.velocity_in().push(velocity, 1);
    if let Some(f) = force_rec {
        circuit.force_in().push(f, 1);
    }

    step_named(&mut circuit, "motion_cases");
    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    match expected_pos {
        Some(expected) => {
            match pos_out.as_slice() {
                [actual] => {
                    assert_eq!(actual.entity, expected.entity);
                    assert_relative_eq!(actual.x.into_inner(), expected.x.into_inner());
                    assert_relative_eq!(actual.y.into_inner(), expected.y.into_inner());
                    assert_relative_eq!(actual.z.into_inner(), expected.z.into_inner());
                }
                [] => panic!("expected a position output"),
                many => panic!("expected one position, observed {}", many.len()),
            }
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
            match vel_out.as_slice() {
                [actual] => {
                    assert_eq!(actual.entity, expected.entity);
                    assert_relative_eq!(actual.vx.into_inner(), expected.vx.into_inner());
                    assert_relative_eq!(actual.vy.into_inner(), expected.vy.into_inner());
                    assert_relative_eq!(actual.vz.into_inner(), expected.vz.into_inner());
                }
                [] => panic!("expected a velocity output"),
                many => panic!("expected one velocity, observed {}", many.len()),
            }
        }
        None => assert!(vel_out.is_empty()),
    }
}

#[rstest]
#[case::positive(1.0)]
#[case::negative(-1.0)]
#[case::zero(0.0)]
fn standing_friction(#[case] vx: f64) {
    let mut circuit = new_circuit();

    circuit.block_in().push(block(1, (0, 0, 0)), 1);
    circuit.block_in().push(block(2, (-1, 0, 0)), 1);
    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(vel(1, (vx, 0.0, 0.0)), 1);

    step_named(&mut circuit, "standing_friction");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    let position = match pos_out.as_slice() {
        [position] => position,
        [] => panic!("expected one position output"),
        many => panic!("expected one position, observed {}", many.len()),
    };
    assert_relative_eq!(position.x.into_inner(), apply_ground_friction(vx));
    assert_relative_eq!(position.y.into_inner(), 0.0);
    assert_relative_eq!(position.z.into_inner(), 1.0);

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    let velocity = match vel_out.as_slice() {
        [velocity] => velocity,
        [] => panic!("expected one velocity output"),
        many => panic!("expected one velocity, observed {}", many.len()),
    };
    assert_relative_eq!(velocity.vx.into_inner(), apply_ground_friction(vx));
    assert_relative_eq!(velocity.vy.into_inner(), 0.0);
    assert_relative_eq!(velocity.vz.into_inner(), 0.0);
}

#[test]
fn airborne_preserves_velocity() {
    let mut circuit = new_circuit();

    circuit.block_in().push(block(1, (0, 0, 0)), 1);
    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 2.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(vel(1, (1.0, 0.0, 0.0)), 1);

    step_named(&mut circuit, "airborne_preserves_velocity");

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    let Some(velocity) = vel_out.first() else {
        panic!("expected a single velocity output");
    };
    assert_relative_eq!(velocity.vx.into_inner(), 1.0);
    assert_relative_eq!(velocity.vy.into_inner(), 0.0);
    assert_relative_eq!(velocity.vz.into_inner(), GRAVITY_PULL);
}

#[rstest]
#[case::at_limit(-TERMINAL_VELOCITY, -TERMINAL_VELOCITY)]
#[case::beyond_limit(-(TERMINAL_VELOCITY + 1.0), -TERMINAL_VELOCITY)]
#[case::upward_limit(TERMINAL_VELOCITY, TERMINAL_VELOCITY + GRAVITY_PULL)]
#[case::upward_beyond_limit(5.0, 5.0 + GRAVITY_PULL)]
#[case::near_zero_negative(-0.0001, -0.0001 + GRAVITY_PULL)]
#[case::near_zero_positive(0.0001, 0.0001 + GRAVITY_PULL)]
fn terminal_velocity_clamping(#[case] start_vz: f64, #[case] expected_vz: f64) {
    let mut circuit = new_circuit();

    circuit.block_in().push(block(1, (0, 0, -10)), 1);
    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 5.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(vel(1, (0.0, 0.0, start_vz)), 1);

    step_named(&mut circuit, "terminal_velocity_clamping");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    let Some(position) = pos_out.first() else {
        panic!("expected a single position output");
    };
    assert_relative_eq!(position.z.into_inner(), 5.0 + expected_vz);

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    let Some(velocity) = vel_out.first() else {
        panic!("expected a single velocity output");
    };
    assert_relative_eq!(velocity.vz.into_inner(), expected_vz);
}

#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "Document DEFAULT_MASS invariant in a focused test"
)]
fn default_mass_is_positive() {
    assert!(crate::DEFAULT_MASS > 0.0, "DEFAULT_MASS must be > 0.0");
}
