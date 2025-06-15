use glam::Vec3;

/// Returns the magnitude of the vector `(x, y, z)`.
pub fn vec_mag(x: f32, y: f32, z: f32) -> f32 {
    Vec3::new(x, y, z).length()
}

/// Normalizes the vector `(x, y, z)`. If the vector is zero, returns `(0, 0, 0)`.
pub fn vec_normalize(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let v = Vec3::new(x, y, z);
    if let Some(n) = v.try_normalize() {
        (n.x, n.y, n.z)
    } else {
        (0.0, 0.0, 0.0)
    }
}
