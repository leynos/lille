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
/// let (ax, ay, az) = applied_acceleration((7.0, -14.0, 21.0), Some(7.0))
///     .expect("valid positive mass yields acceleration");
/// assert!((ax - 1.0).abs() < 1e-6);
/// assert!((ay + 2.0).abs() < 1e-6);
/// assert!((az - 3.0).abs() < 1e-6);
/// ```
#[expect(
    clippy::assertions_on_constants,
    reason = "debug-time validation of default mass"
)]
pub fn applied_acceleration(force: (f64, f64, f64), mass: Option<f64>) -> Option<(f64, f64, f64)> {
    debug_assert!(
        DEFAULT_MASS > MIN_MASS,
        "DEFAULT_MASS must exceed MIN_MASS to avoid unstable defaults",
    );
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

/// Applies ground friction to a horizontal velocity component.
///
/// The returned velocity is reduced by `GROUND_FRICTION` without reversing its
/// direction. The friction constant is clamped to the range `[0.0, 1.0]` at
/// runtime and checked in debug builds to avoid unintended amplification of
/// motion.
///
/// # Examples
///
/// ```rust
/// use lille::{apply_ground_friction, GROUND_FRICTION};
/// let v = 10.0;
/// let expected = v * (1.0 - GROUND_FRICTION);
/// assert!((apply_ground_friction(v) - expected).abs() < 1e-12);
/// ```
#[expect(
    clippy::assertions_on_constants,
    reason = "debug-time validation of ground friction"
)]
pub fn apply_ground_friction(v: f64) -> f64 {
    use crate::GROUND_FRICTION;

    debug_assert!(
        GROUND_FRICTION >= 0.0 && GROUND_FRICTION <= 1.0,
        "GROUND_FRICTION must be within [0,1]",
    );
    let f = GROUND_FRICTION.clamp(0.0, 1.0);
    v * (1.0 - f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GROUND_FRICTION;
    use approx::assert_relative_eq;

    #[test]
    fn friction_scales_velocity() {
        assert_relative_eq!(apply_ground_friction(1.0), 1.0 - GROUND_FRICTION,);
        assert_relative_eq!(apply_ground_friction(-1.0), -(1.0 - GROUND_FRICTION),);
        assert_relative_eq!(apply_ground_friction(0.0), 0.0);
    }

    #[test]
    fn friction_handles_extreme_values() {
        assert_relative_eq!(
            apply_ground_friction(f64::MAX),
            f64::MAX * (1.0 - GROUND_FRICTION),
        );
        assert_relative_eq!(
            apply_ground_friction(f64::MIN),
            f64::MIN * (1.0 - GROUND_FRICTION),
        );
    }
}
