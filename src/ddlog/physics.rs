/// Foreign helper functions used by the DDlog physics rules.
///
/// These functions expose vector math utilities implemented in Rust so that the
/// generated DDlog code can compute vector magnitude and normalisation.
pub mod helpers {
    use glam::Vec3;
    use ordered_float::OrderedFloat;

    /// Returns the magnitude of the vector `(x, y, z)`.
    pub fn vec_mag(
        x: &OrderedFloat<f32>,
        y: &OrderedFloat<f32>,
        z: &OrderedFloat<f32>,
    ) -> OrderedFloat<f32> {
        OrderedFloat(Vec3::new(x.into_inner(), y.into_inner(), z.into_inner()).length())
    }

    /// Returns the unit vector in the direction of `(x, y, z)`.
    /// If the input is not finite or is the zero vector, returns `(0,0,0)`.
    pub fn vec_normalize(
        x: &OrderedFloat<f32>,
        y: &OrderedFloat<f32>,
        z: &OrderedFloat<f32>,
    ) -> ddlog_std::tuple3<OrderedFloat<f32>, OrderedFloat<f32>, OrderedFloat<f32>> {
        let v = Vec3::new(x.into_inner(), y.into_inner(), z.into_inner());
        if !v.is_finite() {
            return ddlog_std::tuple3(OrderedFloat(0.0), OrderedFloat(0.0), OrderedFloat(0.0));
        }
        match v.try_normalize() {
            Some(n) => ddlog_std::tuple3(OrderedFloat(n.x), OrderedFloat(n.y), OrderedFloat(n.z)),
            None => ddlog_std::tuple3(OrderedFloat(0.0), OrderedFloat(0.0), OrderedFloat(0.0)),
        }
    }
}

pub use helpers::{vec_mag, vec_normalize};
