//! Basic vector math helper functions.
//! Small helpers for calculating magnitudes and normalised vectors.
use glam::Vec3;

/// Returns the magnitude of the vector `(x, y, z)`.
pub fn vec_mag(x: f32, y: f32, z: f32) -> f32 {
    Vec3::new(x, y, z).length()
}

/// Returns the unit vector in the direction of `(x, y, z)`, or `(0.0, 0.0, 0.0)` if the input is not a valid non-zero vector.
///
/// The function checks that all components are finite and the vector is non-zero before normalising. If the input is invalid or the zero vector, it returns the zero vector.
///
/// # Examples
///
/// ```
/// use lille::vec_normalize;
/// let (nx, ny, nz) = vec_normalize(3.0, 0.0, 4.0);
/// assert!((nx - 0.6).abs() < 1e-6);
/// assert!((ny - 0.0).abs() < 1e-6);
/// assert!((nz - 0.8).abs() < 1e-6);
///
/// let zero = vec_normalize(0.0, 0.0, 0.0);
/// assert_eq!(zero, (0.0, 0.0, 0.0));
/// ```
pub fn vec_normalize(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let v = Vec3::new(x, y, z);
    if !v.is_finite() {
        return (0.0, 0.0, 0.0);
    }

    let n = v.try_normalize().unwrap_or(Vec3::ZERO);
    (n.x, n.y, n.z)
}
