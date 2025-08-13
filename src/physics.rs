//! Physics helper functions.
//!
//! Provides utilities for basic physics calculations used by the
//! simulation. These functions operate on simple numeric tuples so they
//! can be reused both inside the DBSP circuit and in standalone unit
//! tests.

use crate::DEFAULT_MASS;

/// Smallest acceptable mass to avoid numerically unstable accelerations.
const MIN_MASS: f64 = 1e-12;

/// Computes acceleration from a force vector and optional mass.
///
/// Returns `None` if `mass` is non-positive or effectively zero (see
/// [`MIN_MASS`]). When `mass` is `None` the [`DEFAULT_MASS`] constant is
/// used. The calculation applies `F=ma` for each component independently.
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
        Some(m) if m > MIN_MASS => {
            let (fx, fy, fz) = force;
            Some((fx / m, fy / m, fz / m))
        }
        Some(_) => None,
        None => {
            let (fx, fy, fz) = force;
            Some((fx / DEFAULT_MASS, fy / DEFAULT_MASS, fz / DEFAULT_MASS))
        }
    }
}
