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
