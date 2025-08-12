//! Shared utilities for constructing common physics records in tests.
#![allow(unfulfilled_lint_expectations)]

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

/// Convenience constructor for [`Force`] records without mass used in tests.
#[expect(
    dead_code,
    reason = "Test utility function used across multiple test files"
)]
pub fn force(entity: i64, fx: f64, fy: f64, fz: f64) -> Force {
    Force {
        entity,
        fx: fx.into(),
        fy: fy.into(),
        fz: fz.into(),
        mass: None,
    }
}

/// Convenience constructor for [`Force`] records with an explicit mass used in tests.
#[expect(
    dead_code,
    reason = "Test utility function used across multiple test files"
)]
pub fn force_with_mass(entity: i64, fx: f64, fy: f64, fz: f64, mass: f64) -> Force {
    Force {
        entity,
        fx: fx.into(),
        fy: fy.into(),
        fz: fz.into(),
        mass: Some(mass.into()),
    }
}

/// Convenience constructor for [`Block`] records used in tests.
#[allow(dead_code)]
pub fn block(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}
