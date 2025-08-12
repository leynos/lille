use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, Force, Position, Velocity};

/// Create a new `DbspCircuit` for tests.
///
/// Panics if the circuit cannot be constructed.
#[allow(dead_code)]
pub fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}

/// Convenience constructor for [`Position`] records used in tests.
#[allow(dead_code)]
pub fn pos(entity: i64, x: f64, y: f64, z: f64) -> Position {
    Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

/// Convenience constructor for [`Velocity`] records used in tests.
#[allow(dead_code)]
pub fn vel(entity: i64, vx: f64, vy: f64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: vx.into(),
        vy: vy.into(),
        vz: vz.into(),
    }
}

#[allow(dead_code)]
pub fn force(entity: i64, fx: f64, fy: f64, fz: f64, mass: Option<f64>) -> Force {
    Force {
        entity,
        fx: fx.into(),
        fy: fy.into(),
        fz: fz.into(),
        mass: mass.map(|m| m.into()),
    }
}

/// Convenience constructor for [`Block`] records used in tests.
#[allow(dead_code)]
pub fn block(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}
