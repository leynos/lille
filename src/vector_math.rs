//! Basic vector math helper functions.
//! Small helpers for calculating magnitudes and normalised vectors.
use glam::Vec3;

/// Returns the magnitude of a vector expressed by its components.
///
/// # Examples
/// ```
/// use lille::vector_math::vec_mag;
/// let magnitude = vec_mag(3.0, 4.0, 12.0);
/// assert!((magnitude - 13.0).abs() < f32::EPSILON);
/// ```
pub fn vec_mag(component_x: f32, component_y: f32, component_z: f32) -> f32 {
    Vec3::new(component_x, component_y, component_z).length()
}

/// Returns the unit vector in the direction of the supplied components.
///
/// The function checks that all components are finite and the vector is
/// non-zero before normalising. If the input is invalid or the zero vector,
/// it returns `(0.0, 0.0, 0.0)`.
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
pub fn vec_normalize(component_x: f32, component_y: f32, component_z: f32) -> (f32, f32, f32) {
    let vector = Vec3::new(component_x, component_y, component_z);
    if !vector.is_finite() {
        return (0.0, 0.0, 0.0);
    }

    let normalised = vector.try_normalize().unwrap_or(Vec3::ZERO);
    (normalised.x, normalised.y, normalised.z)
}
