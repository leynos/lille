//! Behavioural tests for reactive agent movement decisions.
//!
//! Verifies that DBSP-derived movement responds to fear and target inputs,
//! ensuring the circuit remains the source of truth for agent behaviour.

use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, FearLevel, NewPosition, Position, Target, Velocity};
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
        reason = "DbspCircuit wrapped in Arc<Mutex<_>> for shared test access",
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

#[test]
fn moves_towards_target_when_unafraid() {
    rspec::run(&rspec::given(
        "an entity with a target and no fear",
        Env::default(),
        |ctx| {
            ctx.before_each(|env| {
                env.push_block(block(1, 0, 0, 0));
                env.push_block(block(2, 1, 1, 0));
                env.push_position(pos(1, 0.0, 0.0, 1.0));
                env.push_velocity(vel(1, 0.0, 0.0, 0.0));
                env.push_target(target(1, 1.0, 1.0));
                env.step();
            });
            ctx.then("it moves towards the target", |env| {
                let out = env.output();
                assert_eq!(
                    out,
                    vec![NewPosition {
                        entity: 1,
                        x: 0.7071067811865475.into(),
                        y: 0.7071067811865475.into(),
                        z: 1.0.into()
                    }],
                );
            });
        },
    ));
}

#[test]
fn flees_target_when_afraid() {
    rspec::run(&rspec::given(
        "an entity with a target and fear above threshold",
        Env::default(),
        |ctx| {
            ctx.before_each(|env| {
                env.push_block(block(1, -1, 0, 0));
                env.push_block(block(2, 0, 0, 0));
                env.push_target(target(1, 1.0, 1.0));
                env.push_position(pos(1, 0.0, 0.0, 1.0));
                env.push_velocity(vel(1, 0.0, 0.0, 0.0));
                env.push_fear(fear(1, 0.5));
                env.step();
            });
            ctx.then("it moves away from the target", |env| {
                let out = env.output();
                assert_eq!(
                    out,
                    vec![NewPosition {
                        entity: 1,
                        x: (-0.7071067811865475).into(),
                        y: (-0.7071067811865475).into(),
                        z: 1.0.into()
                    }],
                );
            });
        },
    ));
}

#[test]
fn no_movement_without_target() {
    rspec::run(&rspec::given(
        "an entity without a target",
        Env::default(),
        |ctx| {
            ctx.before_each(|env| {
                env.push_block(block(1, 0, 0, 0));
                env.push_position(pos(1, 0.0, 0.0, 1.0));
                env.push_velocity(vel(1, 0.0, 0.0, 0.0));
                env.step();
            });
            ctx.then("it remains in place", |env| {
                let out = env.output();
                assert_eq!(
                    out,
                    vec![NewPosition {
                        entity: 1,
                        x: 0.0.into(),
                        y: 0.0.into(),
                        z: 1.0.into()
                    }],
                );
            });
        },
    ));
}
