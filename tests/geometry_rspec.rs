use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::FloorHeightAt,
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
        let circuit = Arc::new(Mutex::new(DbspCircuit::new().expect("c")));
        Self { circuit }
    }
}

impl Env {
    fn push(&self, block: Block, slope: Option<BlockSlope>) {
        let c = &mut *self.circuit.lock().unwrap();
        c.block_in().push(block, 1);
        if let Some(s) = slope {
            c.block_slope_in().push(s, 1);
        }
    }

    fn step(&self) {
        self.circuit.lock().unwrap().step().unwrap();
    }

    fn output(&self) -> Vec<FloorHeightAt> {
        let mut c = self.circuit.lock().unwrap();
        let vals: Vec<_> = c
            .floor_height_out()
            .consolidate()
            .iter()
            .map(|(fh, _, _)| fh.clone())
            .collect();
        c.clear_inputs();
        vals
    }
}

#[test]
fn slope_block_outputs_height() {
    rspec::run(&rspec::given(
        "a block with a slope",
        Env::default(),
        |ctx| {
            ctx.before_each(|env| {
                env.push(
                    Block {
                        id: 1,
                        x: 0,
                        y: 0,
                        z: 0,
                    },
                    Some(BlockSlope {
                        block_id: 1,
                        grad_x: 1.0f64.into(),
                        grad_y: 0.0f64.into(),
                    }),
                );
                env.step();
            });
            ctx.then("floor height reflects the slope", |env| {
                let out = env.output();
                assert_eq!(
                    out,
                    vec![FloorHeightAt {
                        x: 0,
                        y: 0,
                        z: 1.5.into()
                    }]
                );
            });
        },
    ));
}
