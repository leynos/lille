//! Physics behaviour-driven development tests using the DBSP circuit.
//!
//! This module tests physics rules such as gravity effects on entity positions
//! through the declarative dataflow circuit.

use approx::assert_relative_eq;
use lille::dbsp_circuit::{NewPosition, NewVelocity, Position, Velocity};
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
