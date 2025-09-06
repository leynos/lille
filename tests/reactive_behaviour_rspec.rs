//! Behavioural tests for reactive agent movement decisions.
//!
//! Verifies that DBSP-derived movement responds to fear and target inputs,
//! ensuring the circuit remains the source of truth for agent behaviour.

use approx::assert_relative_eq;
use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, FearLevel, NewPosition, Position, Target, Velocity};
use rstest::rstest;
use test_utils::{
    block, fear, pos, step, target, vel, BlockCoords, BlockId, Coords2D, Coords3D, EntityId,
    FearValue,
};

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
        step!(&mut self.circuit);
    }

    fn drain_output(&mut self) -> Vec<NewPosition> {
        let vals: Vec<NewPosition> = self
            .circuit
            .new_position_out()
            .consolidate()
            .iter()
            .map(|(p, _w, _ts)| p)
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
    Some(target(EntityId::new(1), Coords2D::new(1.0, 1.0))),
    vec![NewPosition {
        entity: 1,
        x: std::f64::consts::FRAC_1_SQRT_2.into(),
        y: std::f64::consts::FRAC_1_SQRT_2.into(),
        z: 1.0.into(),
    }],
)]
#[case(
    "flees target when afraid",
    vec![(1, -1, 0, 0), (2, 0, 0, 0)],
    Some(fear(EntityId::new(1), FearValue::new(0.5))),
    Some(target(EntityId::new(1), Coords2D::new(1.0, 1.0))),
    vec![NewPosition {
        entity: 1,
        x: (-std::f64::consts::FRAC_1_SQRT_2).into(),
        y: (-std::f64::consts::FRAC_1_SQRT_2).into(),
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
        env.push_block(block(BlockId::new(entity), BlockCoords::new(x, y, z)));
    }
    env.push_position(pos(EntityId::new(1), Coords3D::new(0.0, 0.0, 1.0)));
    env.push_velocity(vel(EntityId::new(1), Coords3D::new(0.0, 0.0, 0.0)));
    if let Some(t) = target_input {
        env.push_target(t);
    }
    if let Some(f) = fear_input {
        env.push_fear(f);
    }
    env.step();
    let out = env.drain_output();
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
    env.push_block(block(BlockId::new(1), BlockCoords::new(-1, 0, 0)));
    env.push_block(block(BlockId::new(2), BlockCoords::new(0, 0, 0)));
    env.push_block(block(BlockId::new(3), BlockCoords::new(1, 1, 0)));

    env.push_position(pos(EntityId::new(1), Coords3D::new(0.0, 0.0, 1.0)));
    env.push_velocity(vel(EntityId::new(1), Coords3D::new(0.0, 0.0, 0.0)));
    env.push_target(target(EntityId::new(1), Coords2D::new(1.0, 1.0)));
    env.push_fear(fear(EntityId::new(1), FearValue::new(0.5)));

    env.push_position(pos(EntityId::new(2), Coords3D::new(0.0, 0.0, 1.0)));
    env.push_velocity(vel(EntityId::new(2), Coords3D::new(0.0, 0.0, 0.0)));
    env.push_target(target(EntityId::new(2), Coords2D::new(1.0, 1.0)));

    env.push_position(pos(EntityId::new(3), Coords3D::new(0.0, 0.0, 1.0)));
    env.push_velocity(vel(EntityId::new(3), Coords3D::new(0.0, 0.0, 0.0)));

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
    assert_eq!(out.len(), expected.len());
    for (p, (e, x, y, z)) in out.iter().zip(expected) {
        assert_eq!(p.entity, e);
        assert_relative_eq!(p.x.into_inner(), x);
        assert_relative_eq!(p.y.into_inner(), y);
        assert_relative_eq!(p.z.into_inner(), z);
    }
}
