use lille::{
    components::{Block, BlockSlope},
    dbsp_circuit::FloorHeightAt,
};
mod common;
use rstest::rstest;

fn blk(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}

fn slope(block_id: i64, gx: f32, gy: f32) -> BlockSlope {
    BlockSlope {
        block_id,
        grad_x: (gx as f64).into(),
        grad_y: (gy as f64).into(),
    }
}

fn fh(x: i32, y: i32, z: f64) -> FloorHeightAt {
    FloorHeightAt { x, y, z: z.into() }
}

#[rstest]
#[case(vec![blk(1,0,0,0)], vec![], vec![fh(0,0,1.0)])]
#[case(vec![blk(1,0,0,0)], vec![slope(1,1.0,0.0)], vec![fh(0,0,1.5)])]
fn floor_height_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] expected: Vec<FloorHeightAt>,
) {
    let mut circuit = common::new_circuit();
    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    for s in &slopes {
        circuit.block_slope_in().push(s.clone(), 1);
    }
    circuit.step().expect("step");
    let mut vals: Vec<FloorHeightAt> = circuit
        .floor_height_out()
        .consolidate()
        .iter()
        .map(|(fh, _, _)| fh.clone())
        .collect();
    vals.sort_by_key(|h| (h.x, h.y));
    let mut exp = expected;
    exp.sort_by_key(|h| (h.x, h.y));
    assert_eq!(vals, exp);
}
