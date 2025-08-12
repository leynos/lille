//! Physics helper functions.
//!
//! Provides utilities for basic physics calculations used by the
//! simulation. These functions operate on simple numeric tuples so they
//! can be reused both inside the DBSP circuit and in standalone unit
//! tests.

use crate::DEFAULT_MASS;

/// Computes acceleration from a force vector and optional mass.
///
/// Returns `None` if `mass` is non-positive. When `mass` is `None` the
/// [`DEFAULT_MASS`] constant is used. The calculation applies `F=ma` for
/// each component independently.
///
/// # Examples
///
/// ```
/// use lille::applied_acceleration;
/// let (ax, ay, az) = applied_acceleration((7.0, -14.0, 21.0), Some(7.0)).unwrap();
/// assert!((ax - 1.0).abs() < 1e-6);
/// assert!((ay + 2.0).abs() < 1e-6);
/// assert!((az - 3.0).abs() < 1e-6);
/// ```
pub fn applied_acceleration(force: (f64, f64, f64), mass: Option<f64>) -> Option<(f64, f64, f64)> {
    match mass {
        Some(m) if m > 0.0 => Some((force.0 / m, force.1 / m, force.2 / m)),
        Some(_) => None,
        None => Some((
            force.0 / DEFAULT_MASS,
            force.1 / DEFAULT_MASS,
            force.2 / DEFAULT_MASS,
        )),
    }
}
