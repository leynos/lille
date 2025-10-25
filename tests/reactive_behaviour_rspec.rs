//! Behavioural tests for reactive agent movement decisions.
//!
//! Verifies that DBSP-derived movement responds to fear and target inputs,
//! ensuring the circuit remains the source of truth for agent behaviour.

use anyhow::{ensure, Context, Result};
use approx::relative_eq;
use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, FearLevel, NewPosition, Position, Target, Velocity};
use rstest::rstest;
use test_utils::{block, fear, pos, step, vel};

struct Env {
    // Owns the circuit directly so tests can mutate it without synchronisation
    // primitives.
    circuit: DbspCircuit,
}

impl Env {
    fn new() -> Result<Self> {
        let circuit = DbspCircuit::new().context("failed to create DBSP circuit for test")?;
        Ok(Self { circuit })
    }

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
        step(&mut self.circuit);
    }

    fn drain_output(&mut self) -> Vec<NewPosition> {
        let vals: Vec<NewPosition> = self
            .circuit
            .new_position_out()
            .consolidate()
            .iter()
            .map(|(p, (), _timestamp)| p)
            .collect();
        self.circuit.clear_inputs();
        vals
    }
}

#[derive(Clone)]
struct ReactiveScenario {
    description: &'static str,
    blocks: Vec<(i64, i32, i32, i32)>,
    fear_input: Option<FearLevel>,
    target_input: Option<Target>,
    expected_output: Vec<NewPosition>,
}

#[rstest]
#[case(ReactiveScenario {
    description: "moves towards target when unafraid",
    blocks: vec![(1, 0, 0, 0), (2, 1, 1, 0)],
    fear_input: None,
    target_input: Some(Target { entity: 1, x: 1.0.into(), y: 1.0.into() }),
    expected_output: vec![NewPosition {
        entity: 1,
        x: std::f64::consts::FRAC_1_SQRT_2.into(),
        y: std::f64::consts::FRAC_1_SQRT_2.into(),
        z: 1.0.into(),
    }],
})]
#[case(ReactiveScenario {
    description: "flees target when afraid",
    blocks: vec![(1, -1, 0, 0), (2, 0, 0, 0)],
    fear_input: Some(fear(1, 0.5_f32)),
    target_input: Some(Target { entity: 1, x: 1.0.into(), y: 1.0.into() }),
    expected_output: vec![NewPosition {
        entity: 1,
        x: (-std::f64::consts::FRAC_1_SQRT_2).into(),
        y: (-std::f64::consts::FRAC_1_SQRT_2).into(),
        z: 1.0.into(),
    }],
})]
#[case(ReactiveScenario {
    description: "no movement without target",
    blocks: vec![(1, 0, 0, 0)],
    fear_input: None,
    target_input: None,
    expected_output: vec![NewPosition {
        entity: 1,
        x: 0.0.into(),
        y: 0.0.into(),
        z: 1.0.into(),
    }],
})]
fn reactive_movement_behaviour(#[case] scenario: ReactiveScenario) -> Result<()> {
    let ReactiveScenario {
        description,
        blocks,
        fear_input,
        target_input,
        expected_output,
    } = scenario;
    let _ = description;
    let mut env = Env::new()?;
    for (entity, x, y, z) in blocks {
        env.push_block(block(entity, (x, y, z)));
    }
    env.push_position(pos(1, (0.0, 0.0, 1.0)));
    env.push_velocity(vel(1, (0.0, 0.0, 0.0)));
    if let Some(target) = target_input {
        env.push_target(target);
    }
    if let Some(fear) = fear_input {
        env.push_fear(fear);
    }
    env.step();
    let out = env.drain_output();
    ensure!(
        out.len() == expected_output.len(),
        "expected {} positions, observed {}",
        expected_output.len(),
        out.len()
    );
    for (actual, expected) in out.iter().zip(expected_output.iter()) {
        ensure!(
            actual.entity == expected.entity,
            "entity mismatch: expected {}, found {}",
            expected.entity,
            actual.entity
        );
        ensure!(
            relative_eq!(actual.x.into_inner(), expected.x.into_inner()),
            "x mismatch for entity {}",
            actual.entity
        );
        ensure!(
            relative_eq!(actual.y.into_inner(), expected.y.into_inner()),
            "y mismatch for entity {}",
            actual.entity
        );
        ensure!(
            relative_eq!(actual.z.into_inner(), expected.z.into_inner()),
            "z mismatch for entity {}",
            actual.entity
        );
    }
    Ok(())
}

#[test]
fn handles_multiple_entities_with_mixed_states() -> Result<()> {
    let mut env = Env::new()?;
    env.push_block(block(1, (-1, 0, 0)));
    env.push_block(block(2, (0, 0, 0)));
    env.push_block(block(3, (1, 1, 0)));

    env.push_position(pos(1, (0.0, 0.0, 1.0)));
    env.push_velocity(vel(1, (0.0, 0.0, 0.0)));
    env.push_target(Target {
        entity: 1,
        x: 1.0.into(),
        y: 1.0.into(),
    });
    env.push_fear(fear(1, 0.5_f32));

    env.push_position(pos(2, (0.0, 0.0, 1.0)));
    env.push_velocity(vel(2, (0.0, 0.0, 0.0)));
    env.push_target(Target {
        entity: 2,
        x: 1.0.into(),
        y: 1.0.into(),
    });

    env.push_position(pos(3, (0.0, 0.0, 1.0)));
    env.push_velocity(vel(3, (0.0, 0.0, 0.0)));

    env.step();
    let mut out = env.drain_output();
    out.sort_by_key(|p| p.entity);
    let expected = [
        (
            1,
            -std::f64::consts::FRAC_1_SQRT_2,
            -std::f64::consts::FRAC_1_SQRT_2,
            1.0,
        ),
        (
            2,
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            1.0,
        ),
        (3, 0.0, 0.0, 1.0),
    ];
    ensure!(
        out.len() == expected.len(),
        "expected {} entities, observed {}",
        expected.len(),
        out.len()
    );
    for (position, (entity, x, y, z)) in out.iter().zip(expected) {
        ensure!(
            position.entity == entity,
            "entity mismatch: expected {}, found {}",
            entity,
            position.entity
        );
        ensure!(
            relative_eq!(position.x.into_inner(), x),
            "x mismatch for entity {}",
            entity
        );
        ensure!(
            relative_eq!(position.y.into_inner(), y),
            "y mismatch for entity {}",
            entity
        );
        ensure!(
            relative_eq!(position.z.into_inner(), z),
            "z mismatch for entity {}",
            entity
        );
    }
    Ok(())
}
