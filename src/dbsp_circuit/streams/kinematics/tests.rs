use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::{
    DbspCircuit, Force, NewPosition, NewVelocity, Position, PositionFloor, Velocity,
};
use crate::{apply_ground_friction, GRAVITY_PULL, TERMINAL_VELOCITY};
use approx::assert_relative_eq;
use rstest::rstest;

fn slope(block_id: i64, gx: f64, gy: f64) -> BlockSlope {
    BlockSlope {
        block_id,
        grad_x: gx.into(),
        grad_y: gy.into(),
    }
}

fn pf(position: Position, z_floor: f64) -> PositionFloor {
    PositionFloor {
        position,
        z_floor: z_floor.into(),
    }
}

fn block(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}

fn pos(entity: i64, x: f64, y: f64, z: f64) -> Position {
    Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

fn vel(entity: i64, vx: f64, vy: f64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: vx.into(),
        vy: vy.into(),
        vz: vz.into(),
    }
}

fn force(entity: i64, force: (f64, f64, f64)) -> Force {
    Force {
        entity,
        fx: force.0.into(),
        fy: force.1.into(),
        fz: force.2.into(),
        mass: None,
    }
}

fn force_with_mass(entity: i64, force: (f64, f64, f64), mass: f64) -> Force {
    Force {
        entity,
        fx: force.0.into(),
        fy: force.1.into(),
        fz: force.2.into(),
        mass: Some(mass.into()),
    }
}

fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}

#[rstest]
#[case(
    vec![block(1,0,0,0)],
    vec![],
    vec![pos(1,0.2,0.3,2.0)],
    vec![pf(pos(1,0.2,0.3,2.0),1.0)],
)]
#[case(
    vec![],
    vec![],
    vec![pos(1,0.0,0.0,0.5)],
    vec![],
)]
#[case(
    vec![block(1,-1,-1,0)],
    vec![slope(1,1.0,0.0)],
    vec![pos(2,-0.8,-0.2,3.0)],
    vec![pf(pos(2,-0.8,-0.2,3.0),1.5)],
)]
fn position_floor_cases(
    #[case] blocks: Vec<Block>,
    #[case] slopes: Vec<BlockSlope>,
    #[case] positions: Vec<Position>,
    #[case] expected: Vec<PositionFloor>,
) {
    let mut circuit = new_circuit();
    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    for s in &slopes {
        circuit.block_slope_in().push(s.clone(), 1);
    }
    for p in &positions {
        circuit.position_in().push(*p, 1);
    }
    circuit.step().expect("step");
    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, _, _)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);
    let mut exp = expected;
    exp.sort_by_key(|pf| pf.position.entity);
    assert_eq!(vals, exp);
}

#[test]
fn multiple_positions_same_grid_cell() {
    let mut circuit = new_circuit();
    circuit.block_in().push(block(1, 0, 0, 0), 1);
    circuit.position_in().push(pos(1, 0.1, 0.1, 2.0), 1);
    circuit.position_in().push(pos(2, 0.8, 0.4, 3.0), 1);
    circuit.step().expect("step");

    let mut vals: Vec<PositionFloor> = circuit
        .position_floor_out()
        .consolidate()
        .iter()
        .map(|(pf, _, _)| pf.clone())
        .collect();
    vals.sort_by_key(|pf| pf.position.entity);

    let mut exp = vec![
        pf(pos(1, 0.1, 0.1, 2.0), 1.0),
        pf(pos(2, 0.8, 0.4, 3.0), 1.0),
    ];
    exp.sort_by_key(|pf| pf.position.entity);

    assert_eq!(vals, exp);
}

#[rstest]
#[case::standing_moves(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 1.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0), block(2, 1, 0, 1)],
    None,
    Some(Position { entity: 1, x: apply_ground_friction(1.0).into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, apply_ground_friction(1.0), 0.0, 0.0)),
)]
#[case::unsupported_falls(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    None,
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() }),
    Some(vel(1, 0.0, 0.0, GRAVITY_PULL)),
)]
#[case::boundary_snaps_to_floor(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    None,
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, 0.0, 0.0, 0.0)),
)]
#[case::force_accelerates(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0), block(2, 1, 0, 1)],
    Some(force_with_mass(1, (5.0, 0.0, 0.0), 5.0)),
    Some(Position { entity: 1, x: apply_ground_friction(1.0).into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, apply_ground_friction(1.0), 0.0, 0.0)),
)]
#[case::invalid_mass_ignores_force(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 2.1.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    Some(force_with_mass(1, (0.0, 0.0, 10.0), 0.0)),
    Some(Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.1.into() }),
    Some(vel(1, 0.0, 0.0, GRAVITY_PULL)),
)]
#[case::force_with_default_mass(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    vel(1, 0.0, 0.0, 0.0),
    vec![block(1, 0, 0, 0)],
    Some(force(1, (crate::DEFAULT_MASS, 0.0, 0.0))),
    Some(Position { entity: 1, x: apply_ground_friction(1.0).into(), y: 0.0.into(), z: 1.0.into() }),
    Some(vel(1, apply_ground_friction(1.0), 0.0, 0.0)),
)]
fn motion_cases(
    #[case] position: Position,
    #[case] velocity: Velocity,
    #[case] blocks: Vec<Block>,
    #[case] force_rec: Option<Force>,
    #[case] expected_pos: Option<NewPosition>,
    #[case] expected_vel: Option<NewVelocity>,
) {
    let mut circuit = new_circuit();

    for b in &blocks {
        circuit.block_in().push(b.clone(), 1);
    }
    circuit.position_in().push(position, 1);
    circuit.velocity_in().push(velocity, 1);
    if let Some(f) = force_rec {
        circuit.force_in().push(f, 1);
    }

    circuit.step().expect("circuit step failed");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    match expected_pos {
        Some(expected) => {
            assert_eq!(pos_out.len(), 1);
            assert_eq!(pos_out[0].entity, expected.entity);
            assert_relative_eq!(pos_out[0].x.into_inner(), expected.x.into_inner());
            assert_relative_eq!(pos_out[0].y.into_inner(), expected.y.into_inner());
            assert_relative_eq!(pos_out[0].z.into_inner(), expected.z.into_inner());
        }
        None => assert!(pos_out.is_empty()),
    }

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    match expected_vel {
        Some(expected) => {
            assert_eq!(vel_out.len(), 1);
            assert_eq!(vel_out[0].entity, expected.entity);
            assert_relative_eq!(vel_out[0].vx.into_inner(), expected.vx.into_inner());
            assert_relative_eq!(vel_out[0].vy.into_inner(), expected.vy.into_inner());
            assert_relative_eq!(vel_out[0].vz.into_inner(), expected.vz.into_inner());
        }
        None => assert!(vel_out.is_empty()),
    }
}

#[rstest]
#[case::positive(1.0)]
#[case::negative(-1.0)]
#[case::zero(0.0)]
fn standing_friction(#[case] vx: f64) {
    let mut circuit = new_circuit();

    circuit.block_in().push(block(1, 0, 0, 0), 1);
    circuit.block_in().push(block(2, -1, 0, 0), 1);
    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(vel(1, vx, 0.0, 0.0), 1);

    circuit.step().expect("circuit step failed");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    assert_eq!(pos_out.len(), 1);
    assert_relative_eq!(pos_out[0].x.into_inner(), apply_ground_friction(vx));
    assert_relative_eq!(pos_out[0].y.into_inner(), 0.0);
    assert_relative_eq!(pos_out[0].z.into_inner(), 1.0);

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    assert_eq!(vel_out.len(), 1);
    assert_relative_eq!(vel_out[0].vx.into_inner(), apply_ground_friction(vx));
    assert_relative_eq!(vel_out[0].vy.into_inner(), 0.0);
    assert_relative_eq!(vel_out[0].vz.into_inner(), 0.0);
}

#[test]
fn airborne_preserves_velocity() {
    let mut circuit = new_circuit();

    circuit.block_in().push(block(1, 0, 0, 0), 1);
    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 2.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(vel(1, 1.0, 0.0, 0.0), 1);

    circuit.step().expect("circuit step failed");

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    assert_eq!(vel_out.len(), 1);
    assert_relative_eq!(vel_out[0].vx.into_inner(), 1.0);
    assert_relative_eq!(vel_out[0].vy.into_inner(), 0.0);
    assert_relative_eq!(vel_out[0].vz.into_inner(), GRAVITY_PULL);
}

#[rstest]
#[case::at_limit(-TERMINAL_VELOCITY, -TERMINAL_VELOCITY)]
#[case::beyond_limit(-5.0, -TERMINAL_VELOCITY)]
#[case::upward_limit(TERMINAL_VELOCITY, TERMINAL_VELOCITY + GRAVITY_PULL)]
#[case::upward_beyond_limit(5.0, 5.0 + GRAVITY_PULL)]
#[case::near_zero_negative(-0.0001, -0.0001 + GRAVITY_PULL)]
#[case::near_zero_positive(0.0001, 0.0001 + GRAVITY_PULL)]
fn terminal_velocity_clamping(#[case] start_vz: f64, #[case] expected_vz: f64) {
    let mut circuit = new_circuit();

    circuit.block_in().push(block(1, 0, 0, -10), 1);
    circuit.position_in().push(
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 5.0.into(),
        },
        1,
    );
    circuit.velocity_in().push(vel(1, 0.0, 0.0, start_vz), 1);

    circuit.step().expect("circuit step failed");

    let pos_out: Vec<NewPosition> = circuit
        .new_position_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    assert_eq!(pos_out.len(), 1);
    assert_relative_eq!(pos_out[0].z.into_inner(), 5.0 + expected_vz);

    let vel_out: Vec<NewVelocity> = circuit
        .new_velocity_out()
        .consolidate()
        .iter()
        .map(|t| t.0)
        .collect();
    assert_eq!(vel_out.len(), 1);
    assert_relative_eq!(vel_out[0].vz.into_inner(), expected_vz);
}
