//! Behaviour tests for joining positions with floor height.
use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::{Position, PositionFloor},
    DbspCircuit,
};
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
#[allow(clippy::arc_with_non_send_sync)]
struct Env {
    circuit: Arc<Mutex<DbspCircuit>>,
}

unsafe impl Send for Env {}
unsafe impl Sync for Env {}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Env").finish()
    }
}

impl Default for Env {
    fn default() -> Self {
        #[allow(clippy::arc_with_non_send_sync)]
        let circuit = Arc::new(Mutex::new(DbspCircuit::new().expect("create")));
        Self { circuit }
    }
}

impl Env {
    fn push_block(&self, block: Block, slope: Option<BlockSlope>) {
        let c = self.circuit.lock().expect("lock");
        c.block_in().push(block, 1);
        if let Some(s) = slope {
            c.block_slope_in().push(s, 1);
        }
    }

    fn push_position(&self, pos: Position) {
        let c = self.circuit.lock().expect("lock");
        c.position_in().push(pos, 1);
    }

    fn step(&self) {
        self.circuit.lock().expect("lock").step().expect("step");
    }

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

fn pos(entity: i64, x: f64, y: f64, z: f64) -> Position {
    Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
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
                env.push_position(pos(1, 0.2, 0.2, 2.0));
                env.step();
            });
            ctx.then("position is paired with floor height", |env| {
                let out = env.output();
                assert_eq!(
                    out,
                    vec![PositionFloor {
                        position: pos(1, 0.2, 0.2, 2.0),
                        z_floor: 1.0.into(),
                    }]
                );
            });
        },
    ));
}
