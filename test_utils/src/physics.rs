//! Convenience constructors for physics-related records used in tests.

use lille::components::Block;
use lille::dbsp_circuit::{DbspCircuit, Force, Position, Velocity};

/// Create a new [`DbspCircuit`] for tests.
///
/// # Panics
/// Panics if the circuit cannot be constructed.
///
/// # Examples
/// ```no_run
/// use test_utils::physics::new_circuit;
/// let circuit = new_circuit();
/// drop(circuit);
/// ```
pub fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}

/// Convenience constructor for [`Position`] records used in tests.
///
/// # Examples
/// ```
/// use test_utils::physics::pos;
/// let p = pos(1, 0.0, 1.0, 2.0);
/// assert_eq!(p.entity, 1);
/// assert_eq!(p.x.into_inner(), 0.0);
/// assert_eq!(p.y.into_inner(), 1.0);
/// assert_eq!(p.z.into_inner(), 2.0);
/// ```
pub fn pos(entity: i64, x: f64, y: f64, z: f64) -> Position {
    Position {
        entity,
        x: x.into(),
        y: y.into(),
        z: z.into(),
    }
}

/// Convenience constructor for [`Velocity`] records used in tests.
///
/// # Examples
/// ```
/// use test_utils::physics::vel;
/// let v = vel(1, 0.5, -0.5, 1.0);
/// assert_eq!(v.entity, 1);
/// assert_eq!(v.vx.into_inner(), 0.5);
/// assert_eq!(v.vy.into_inner(), -0.5);
/// assert_eq!(v.vz.into_inner(), 1.0);
/// ```
pub fn vel(entity: i64, vx: f64, vy: f64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: vx.into(),
        vy: vy.into(),
        vz: vz.into(),
    }
}

/// Convenience constructor for [`Force`] records without mass used in tests.
///
/// # Examples
/// ```
/// use test_utils::physics::force;
/// let f = force(1, (10.0, 0.0, 0.0));
/// assert_eq!(f.entity, 1);
/// assert!(f.mass.is_none());
/// ```
fn force_inner(entity: i64, (fx, fy, fz): (f64, f64, f64), mass: Option<f64>) -> Force {
    Force {
        entity,
        fx: fx.into(),
        fy: fy.into(),
        fz: fz.into(),
        mass: mass.map(Into::into),
    }
}

pub fn force(entity: i64, force: (f64, f64, f64)) -> Force {
    force_inner(entity, force, None)
}

/// Convenience constructor for [`Force`] records with an explicit mass used in
/// tests.
///
/// # Examples
/// ```
/// use test_utils::physics::force_with_mass;
/// let f = force_with_mass(1, (10.0, 0.0, 0.0), 5.0);
/// assert_eq!(f.entity, 1);
/// assert_eq!(f.mass.unwrap().into_inner(), 5.0);
/// ```
pub fn force_with_mass(entity: i64, force: (f64, f64, f64), mass: f64) -> Force {
    force_inner(entity, force, Some(mass))
}

/// Convenience constructor for [`Block`] records used in tests.
///
/// # Examples
/// ```
/// use test_utils::physics::block;
/// let b = block(1, 0, 0, 0);
/// assert_eq!(b.id, 1);
/// assert_eq!(b.x, 0);
/// assert_eq!(b.y, 0);
/// assert_eq!(b.z, 0);
/// ```
pub fn block(id: i64, x: i32, y: i32, z: i32) -> Block {
    Block { id, x, y, z }
}
