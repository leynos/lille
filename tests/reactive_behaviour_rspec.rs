//! Behavioural tests for reactive agent movement decisions.
//!
//! Verifies that DBSP-derived movement responds to fear and target inputs,
//! ensuring the circuit remains the source of truth for agent behaviour.

use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, FearLevel, NewPosition, Position, Target, Velocity};
use rstest::rstest;
use std::fmt;
use std::sync::{Arc, Mutex};
use test_utils::{block, fear, pos, target, vel};

#[derive(Clone)]
struct Env {
    circuit: Arc<Mutex<DbspCircuit>>,
}

// SAFETY: DbspCircuit is Send + Sync when wrapped by Arc<Mutex<_>>.
unsafe impl Send for Env {}
unsafe impl Sync for Env {}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Env").finish()
    }
}

impl Default for Env {
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "DbspCircuit wrapped in Arc<Mutex<_>> for shared test access"
    )]
    fn default() -> Self {
        let circuit = Arc::new(Mutex::new(
            DbspCircuit::new().expect("failed to create DBSP circuit for test"),
        ));
        Self { circuit }
    }
}

impl Env {
    fn push_block(&self, b: Block) {
        let c = self.circuit.lock().unwrap();
        c.block_in().push(b, 1);
    }

    fn push_position(&self, p: Position) {
        let c = self.circuit.lock().unwrap();
        c.position_in().push(p, 1);
    }

    fn push_velocity(&self, v: Velocity) {
        let c = self.circuit.lock().unwrap();
        c.velocity_in().push(v, 1);
    }

    fn push_target(&self, t: Target) {
        let c = self.circuit.lock().unwrap();
        c.target_in().push(t, 1);
    }

    fn push_fear(&self, f: FearLevel) {
        let c = self.circuit.lock().unwrap();
        c.fear_in().push(f, 1);
    }

    fn step(&self) {
        self.circuit.lock().unwrap().step().unwrap();
    }

    fn output(&self) -> Vec<NewPosition> {
        let mut c = self.circuit.lock().unwrap();
        let vals: Vec<_> = c
            .new_position_out()
            .consolidate()
            .iter()
            .map(|(p, _, _)| p)
            .collect();
        c.clear_inputs();
        vals
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
        x: 0.7071067811865475.into(),
        y: 0.7071067811865475.into(),
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
        x: (-0.7071067811865475).into(),
        y: (-0.7071067811865475).into(),
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
    #[case] description: &str,
    #[case] blocks: Vec<(u32, i32, i32, i32)>,
    #[case] fear_input: Option<FearLevel>,
    #[case] target_input: Option<Target>,
    #[case] expected_output: Vec<NewPosition>,
) {
    rspec::run(&rspec::given(description, Env::default(), |ctx| {
        let blocks = blocks.clone();
        let target_input = target_input.clone();
        let fear_input = fear_input.clone();
        ctx.before_each(move |env| {
            for (entity, x, y, z) in &blocks {
                env.push_block(block(*entity, *x, *y, *z));
            }
            env.push_position(pos(1, 0.0, 0.0, 1.0));
            env.push_velocity(vel(1, 0.0, 0.0, 0.0));
            if let Some(t) = target_input.clone() {
                env.push_target(t);
            }
            if let Some(f) = fear_input.clone() {
                env.push_fear(f);
            }
            env.step();
        });
        let expected = expected_output.clone();
        ctx.then("it yields expected positions", move |env| {
            let out = env.output();
            assert_eq!(out, expected);
        });
    }));
}

#[test]
fn handles_multiple_entities_with_mixed_states() {
    rspec::run(&rspec::given(
        "multiple entities with mixed fear and target states",
        Env::default(),
        |ctx| {
            ctx.before_each(|env| {
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
            });
            ctx.then("each reacts independently", |env| {
                let mut out = env.output();
                out.sort_by_key(|p| p.entity);
                assert_eq!(
                    out,
                    vec![
                        NewPosition {
                            entity: 1,
                            x: (-0.7071067811865475).into(),
                            y: (-0.7071067811865475).into(),
                            z: 1.0.into(),
                        },
                        NewPosition {
                            entity: 2,
                            x: 0.7071067811865475.into(),
                            y: 0.7071067811865475.into(),
                            z: 1.0.into(),
                        },
                        NewPosition {
                            entity: 3,
                            x: 0.0.into(),
                            y: 0.0.into(),
                            z: 1.0.into(),
                        },
                    ],
                );
            });
        },
    ));
}
