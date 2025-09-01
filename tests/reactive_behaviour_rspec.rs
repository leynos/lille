//! Behavioural tests for reactive agent movement decisions.
//!
//! Verifies that DBSP-derived movement responds to fear and target inputs,
//! ensuring the circuit remains the source of truth for agent behaviour.

use approx::assert_relative_eq;
use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, FearLevel, NewPosition, Position, Target, Velocity};
use rstest::rstest;
use test_utils::{block, fear, pos, target, vel};

struct Env {
    // Owns the circuit directly so tests can mutate it without synchronisation
    // primitives.
    circuit: DbspCircuit,
}

impl Env {
    fn push_block(&mut self, b: Block) {
        self.circuit.block_in().push(b, 1);
    }

    fn push_position(&mut self, p: Position) {
        self.circuit.position_in().push(p, 1);
    }

    fn push_velocity(&mut self, v: Velocity) {
        self.circuit.velocity_in().push(v, 1);
    }

    fn push_target(&mut self, t: Target) {
        self.circuit.target_in().push(t, 1);
    }

    fn push_fear(&mut self, f: FearLevel) {
        self.circuit.fear_in().push(f, 1);
    }

    fn step(&mut self) {
        self.circuit.step().expect("dbsp step");
    }

    fn output(&mut self) -> Vec<NewPosition> {
        let vals: Vec<NewPosition> = self
            .circuit
            .new_position_out()
            .consolidate()
            .iter()
            // Copy the `NewPosition` out of the tuple to return owned values.
            .map(|t| t.0)
            .collect();
        self.circuit.clear_inputs();
        vals
    }
}

impl Default for Env {
    fn default() -> Self {
        Self {
            circuit: DbspCircuit::new().expect("failed to create DBSP circuit for test"),
        }
    }
}

#[rstest]
#[case(
    "moves towards target when unafraid",
    vec![(1, 0, 0, 0), (2, 1, 1, 0)],
    None,
    Some(target(1, 1.0, 1.0)),
    vec![NewPosition {
        entity: 1,
        x: 0.707_106_781_186_547_5.into(),
        y: 0.707_106_781_186_547_5.into(),
        z: 1.0.into(),
    }],
)]
#[case(
    "flees target when afraid",
    vec![(1, -1, 0, 0), (2, 0, 0, 0)],
    Some(fear(1, 0.5)),
    Some(target(1, 1.0, 1.0)),
    vec![NewPosition {
        entity: 1,
        x: (-0.707_106_781_186_547_5).into(),
        y: (-0.707_106_781_186_547_5).into(),
        z: 1.0.into(),
    }],
)]
#[case(
    "no movement without target",
    vec![(1, 0, 0, 0)],
    None,
    None,
    vec![NewPosition {
        entity: 1,
        x: 0.0.into(),
        y: 0.0.into(),
        z: 1.0.into(),
    }],
)]
fn reactive_movement_behaviour(
    #[case] _description: &str,
    #[case] blocks: Vec<(i64, i32, i32, i32)>,
    #[case] fear_input: Option<FearLevel>,
    #[case] target_input: Option<Target>,
    #[case] expected_output: Vec<NewPosition>,
) {
    let mut env = Env::default();
    for (entity, x, y, z) in blocks {
        env.push_block(block(entity, x, y, z));
    }
    env.push_position(pos(1, 0.0, 0.0, 1.0));
    env.push_velocity(vel(1, 0.0, 0.0, 0.0));
    if let Some(t) = target_input {
        env.push_target(t);
    }
    if let Some(f) = fear_input {
        env.push_fear(f);
    }
    env.step();
    let out = env.output();
    assert_eq!(out.len(), expected_output.len());
    for (actual, expected) in out.iter().zip(expected_output.iter()) {
        assert_eq!(actual.entity, expected.entity);
        assert_relative_eq!(actual.x.into_inner(), expected.x.into_inner());
        assert_relative_eq!(actual.y.into_inner(), expected.y.into_inner());
        assert_relative_eq!(actual.z.into_inner(), expected.z.into_inner());
    }
}

#[test]
fn handles_multiple_entities_with_mixed_states() {
    let mut env = Env::default();
    env.push_block(block(1, -1, 0, 0));
    env.push_block(block(2, 0, 0, 0));
    env.push_block(block(3, 1, 1, 0));

    env.push_position(pos(1, 0.0, 0.0, 1.0));
    env.push_velocity(vel(1, 0.0, 0.0, 0.0));
    env.push_target(target(1, 1.0, 1.0));
    env.push_fear(fear(1, 0.5));

    env.push_position(pos(2, 0.0, 0.0, 1.0));
    env.push_velocity(vel(2, 0.0, 0.0, 0.0));
    env.push_target(target(2, 1.0, 1.0));

    env.push_position(pos(3, 0.0, 0.0, 1.0));
    env.push_velocity(vel(3, 0.0, 0.0, 0.0));

    env.step();
    let mut out = env.output();
    out.sort_by_key(|p| p.entity);
    let expected = [
        NewPosition {
            entity: 1,
            x: (-0.707_106_781_186_547_5).into(),
            y: (-0.707_106_781_186_547_5).into(),
            z: 1.0.into(),
        },
        NewPosition {
            entity: 2,
            x: 0.707_106_781_186_547_5.into(),
            y: 0.707_106_781_186_547_5.into(),
            z: 1.0.into(),
        },
        NewPosition {
            entity: 3,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
    ];
    assert_eq!(out.len(), expected.len());
    for (actual, exp) in out.iter().zip(expected.iter()) {
        assert_eq!(actual.entity, exp.entity);
        assert_relative_eq!(actual.x.into_inner(), exp.x.into_inner());
        assert_relative_eq!(actual.y.into_inner(), exp.y.into_inner());
        assert_relative_eq!(actual.z.into_inner(), exp.z.into_inner());
    }
}
