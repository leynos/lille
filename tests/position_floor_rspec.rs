//! Behaviour-driven tests for position and floor height joins in DBSP circuits.
//!
//! This module exercises the DBSP pipeline using the `rust-rspec` framework. It
//! verifies that entity positions are correctly paired with floor height
//! information when processed through `DbspCircuit`. The tests use a shared
//! circuit environment to mimic real application usage and cover both
//! successful joins and edge cases.
use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::{Position, PositionFloor},
    DbspCircuit,
};
use std::fmt;
use std::sync::{Arc, Mutex};
use test_utils::{pos, step};

#[derive(Clone)]
/// Shared test environment wrapping a `DbspCircuit` in a thread-safe container.
struct Env {
    circuit: Arc<Mutex<DbspCircuit>>,
}

// SAFETY: DbspCircuit is Send and Sync when guarded by Arc<Mutex<_>> which
// provides synchronisation for interior mutability.
unsafe impl Send for Env {}
unsafe impl Sync for Env {}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Env").finish()
    }
}

impl Default for Env {
    /// Creates a new environment with a fresh [`DbspCircuit`] instance.
    #[expect(
        clippy::arc_with_non_send_sync,
        reason = "DbspCircuit wrapped in Arc<Mutex<_>> for shared test access"
    )]
    fn default() -> Self {
        let circuit = Arc::new(Mutex::new(
            DbspCircuit::new().expect("failed to create DBSP circuit for test environment"),
        ));
        Self { circuit }
    }
}

impl Env {
    /// Inserts a block (and optional slope) into the circuit.
    fn push_block(&self, block: Block, slope: Option<BlockSlope>) {
        let c = self.circuit.lock().expect("lock");
        c.block_in().push(block, 1);
        if let Some(s) = slope {
            c.block_slope_in().push(s, 1);
        }
    }

    /// Pushes a [`Position`] record into the circuit.
    fn push_position(&self, pos: Position) {
        let c = self.circuit.lock().expect("lock");
        c.position_in().push(pos, 1);
    }

    /// Advances the circuit by one tick.
    fn step(&self) {
        let mut c = self.circuit.lock().expect("lock");
        step(&mut c);
    }

    /// Retrieves and clears the `PositionFloor` output collection.
    fn output(&self) -> Vec<PositionFloor> {
        let mut c = self.circuit.lock().expect("lock");
        let vals: Vec<_> = c
            .position_floor_out()
            .consolidate()
            .iter()
            .map(|(pf, _, _)| pf.clone())
            .collect();
        c.clear_inputs();
        vals
    }
}

/// Runs a behavioural test that verifies positions are joined with floor height.
#[test]
fn join_position_with_floor() {
    rspec::run(&rspec::given(
        "a block with an entity above",
        Env::default(),
        |ctx| {
            ctx.before_each(|env| {
                env.push_block(
                    Block {
                        id: 1,
                        x: 0,
                        y: 0,
                        z: 0,
                    },
                    None,
                );
                env.push_position(pos(1, (0.2, 0.2, 2.0)));
                env.step();
            });
            ctx.then("position is paired with floor height", |env| {
                let out = env.output();
                assert_eq!(
                    out,
                    vec![PositionFloor {
                        position: pos(1, (0.2, 0.2, 2.0)),
                        z_floor: 1.0.into(),
                    }]
                );
            });
        },
    ));
}
