//! Shared constructors for physics records used in tests.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::{DbspCircuit, Force, Position, Velocity};

pub fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}

pub fn block(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}

pub fn slope(block_id: i64, gx: f64, gy: f64) -> BlockSlope {
    BlockSlope {
        block_id,
        grad_x: gx.into(),
        grad_y: gy.into(),
    }
}

pub fn pos(entity: i64, x: f64, y: f64, z: f64) -> Position {
    Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

pub fn vel(entity: i64, vx: f64, vy: f64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: vx.into(),
        vy: vy.into(),
        vz: vz.into(),
    }
}

pub fn force(entity: i64, force: (f64, f64, f64)) -> Force {
    Force {
        entity,
        fx: force.0.into(),
        fy: force.1.into(),
        fz: force.2.into(),
        mass: None,
    }
}

pub fn force_with_mass(entity: i64, force: (f64, f64, f64), mass: f64) -> Force {
    Force {
        entity,
        fx: force.0.into(),
        fy: force.1.into(),
        fz: force.2.into(),
        mass: Some(mass.into()),
    }
}
