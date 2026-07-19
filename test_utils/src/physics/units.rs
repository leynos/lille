//! Newtype wrappers for physics test values.
//!
//! These wrappers give test constructors typed parameters instead of bare
//! primitives, keeping call sites self-describing.

use ordered_float::OrderedFloat;

#[derive(Clone, Copy, Debug)]
pub struct EntityId(pub i64);

impl EntityId {
    /// Create a new [`EntityId`].
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::EntityId;
    /// let id = EntityId::new(1);
    /// assert_eq!(id.0, 1);
    /// ```
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockId(pub i64);

impl BlockId {
    /// Create a new [`BlockId`].
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::BlockId;
    /// let id = BlockId::new(1);
    /// assert_eq!(id.0, 1);
    /// ```
    pub fn new(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Coords3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Coords3D {
    /// Create new 3D coordinates.
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::Coords3D;
    /// let c = Coords3D::new(1.0, 2.0, 3.0);
    /// assert_eq!(c.x, 1.0);
    /// ```
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockCoords {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockCoords {
    /// Create new block coordinates.
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::BlockCoords;
    /// let c = BlockCoords::new(1, 2, 3);
    /// assert_eq!(c.x, 1);
    /// ```
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Coords2D {
    pub x: f64,
    pub y: f64,
}

impl Coords2D {
    /// Create new 2D coordinates.
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::Coords2D;
    /// let c = Coords2D::new(1.0, 2.0);
    /// assert_eq!(c.x, 1.0);
    /// ```
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ForceVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl ForceVector {
    /// Create a new [`ForceVector`].
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::ForceVector;
    /// let f = ForceVector::new(1.0, 0.0, 0.0);
    /// assert_eq!(f.x, 1.0);
    /// ```
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Gradient {
    pub x: f64,
    pub y: f64,
}

impl Gradient {
    /// Create a new [`Gradient`].
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::Gradient;
    /// let g = Gradient::new(1.0, 0.5);
    /// assert_eq!(g.x, 1.0);
    /// ```
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mass(pub f64);
impl Mass {
    /// Create a new [`Mass`].
    ///
    /// # Examples
    /// ```
    /// use ordered_float::OrderedFloat;
    /// use test_utils::physics::Mass;
    /// let m = Mass::new(5.0);
    /// let val: OrderedFloat<f64> = m.into();
    /// assert_eq!(val, OrderedFloat(5.0));
    /// ```
    pub fn new(val: f64) -> Self {
        Self(val)
    }
}

impl From<Mass> for OrderedFloat<f64> {
    fn from(mass: Mass) -> Self {
        let Mass(inner) = mass;
        OrderedFloat(inner)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FearValue(pub f64);
impl FearValue {
    /// Create a new [`FearValue`].
    ///
    /// # Examples
    /// ```
    /// use test_utils::physics::FearValue;
    /// let f = FearValue::new(0.5);
    /// assert_eq!(f.0, 0.5);
    /// ```
    pub fn new(val: f64) -> Self {
        Self(val)
    }
}

#[cfg(test)]
mod tests {
    //! Tests for the physics newtype constructors.
    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use super::*;

    /// Asserts that a scalar newtype constructor stores its value verbatim.
    /// A macro keeps panic locations in the calling test.
    macro_rules! assert_scalar_newtype_stores {
        ($ty:ident, $value:expr) => {{
            let wrapped = $ty::new($value);
            assert_eq!(wrapped.0, $value);
        }};
    }

    #[rstest]
    fn entity_id_stores_value() {
        assert_scalar_newtype_stores!(EntityId, 7_i64);
    }

    #[rstest]
    fn block_id_stores_value() {
        assert_scalar_newtype_stores!(BlockId, 9_i64);
    }

    #[rstest]
    fn mass_stores_value() {
        assert_scalar_newtype_stores!(Mass, 5.5_f64);
    }

    #[rstest]
    fn fear_value_stores_value() {
        assert_scalar_newtype_stores!(FearValue, 0.25_f64);
    }

    #[rstest]
    fn coords3d_stores_components() {
        let coords = Coords3D::new(1.0, 2.0, 3.0);
        assert_eq!((coords.x, coords.y, coords.z), (1.0, 2.0, 3.0));
    }

    #[rstest]
    fn block_coords_stores_components() {
        let coords = BlockCoords::new(4, 5, 6);
        assert_eq!((coords.x, coords.y, coords.z), (4, 5, 6));
    }

    #[rstest]
    fn coords2d_stores_components() {
        let coords = Coords2D::new(7.0, 8.0);
        assert_eq!((coords.x, coords.y), (7.0, 8.0));
    }

    #[rstest]
    fn force_vector_stores_components() {
        let force = ForceVector::new(1.0, -2.0, 0.5);
        assert_eq!((force.x, force.y, force.z), (1.0, -2.0, 0.5));
    }

    #[rstest]
    fn gradient_stores_components() {
        let gradient = Gradient::new(0.5, -0.5);
        assert_eq!((gradient.x, gradient.y), (0.5, -0.5));
    }

    #[rstest]
    fn mass_converts_to_ordered_float() {
        let value: OrderedFloat<f64> = Mass::new(2.5).into();
        assert_eq!(value, OrderedFloat(2.5));
    }
}
