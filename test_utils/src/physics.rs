//! Convenience constructors for physics-related records used in tests.

use lille::components::{Block, BlockSlope};
use lille::dbsp_circuit::{DbspCircuit, FearLevel, Force, Position, Target, Velocity};

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
    /// use test_utils::physics::Mass;
    /// let m = Mass::new(5.0);
    /// assert_eq!(m.0, 5.0);
    /// ```
    pub fn new(val: f64) -> Self {
        Self(val)
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

/// Create a new [`DbspCircuit`] for tests.
///
/// # Panics
/// Panics if the circuit cannot be constructed.
///
/// # Examples
/// ```rust,no_run
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
/// ```rust
/// use test_utils::physics::{pos, EntityId, Coords3D};
/// let p = pos(EntityId::new(1), Coords3D::new(0.0, 1.0, 2.0));
/// assert_eq!(p.entity, 1);
/// assert_eq!(p.x.into_inner(), 0.0);
/// assert_eq!(p.y.into_inner(), 1.0);
/// assert_eq!(p.z.into_inner(), 2.0);
/// ```
pub fn pos(entity: EntityId, coords: Coords3D) -> Position {
    Position {
        entity: entity.0,
        x: coords.x.into(),
        y: coords.y.into(),
        z: coords.z.into(),
    }
}

/// Convenience constructor for [`Velocity`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{vel, EntityId, Coords3D};
/// let v = vel(EntityId::new(1), Coords3D::new(0.5, -0.5, 1.0));
/// assert_eq!(v.entity, 1);
/// assert_eq!(v.vx.into_inner(), 0.5);
/// assert_eq!(v.vy.into_inner(), -0.5);
/// assert_eq!(v.vz.into_inner(), 1.0);
/// ```
pub fn vel(entity: EntityId, velocity: Coords3D) -> Velocity {
    Velocity {
        entity: entity.0,
        vx: velocity.x.into(),
        vy: velocity.y.into(),
        vz: velocity.z.into(),
    }
}

/// Convenience constructor for [`Target`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{target, EntityId, Coords2D};
/// let t = target(EntityId::new(1), Coords2D::new(1.0, 2.0));
/// assert_eq!(t.entity, 1);
/// assert_eq!(t.x.into_inner(), 1.0);
/// assert_eq!(t.y.into_inner(), 2.0);
/// ```
#[inline]
pub fn target(entity: EntityId, coords: Coords2D) -> Target {
    Target {
        entity: entity.0,
        x: coords.x.into(),
        y: coords.y.into(),
    }
}

/// Convenience constructor for [`FearLevel`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{fear, EntityId, FearValue};
/// let f = fear(EntityId::new(1), FearValue::new(0.5));
/// assert_eq!(f.entity, 1);
/// assert_eq!(f.level.into_inner(), 0.5);
/// ```
#[inline]
pub fn fear(entity: EntityId, level: FearValue) -> FearLevel {
    FearLevel {
        entity: entity.0,
        level: level.0.into(),
    }
}

/// Convenience constructor for [`Force`] records without mass used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{force, EntityId, ForceVector};
/// let f = force(EntityId::new(1), ForceVector::new(10.0, 0.0, 0.0));
/// assert_eq!(f.entity, 1);
/// assert!(f.mass.is_none());
/// ```
fn force_inner(entity: EntityId, vec: ForceVector, mass: Option<Mass>) -> Force {
    Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: mass.map(|m| m.0.into()),
    }
}

pub fn force(entity: EntityId, vec: ForceVector) -> Force {
    force_inner(entity, vec, None)
}

/// Convenience constructor for [`Force`] records with an explicit mass used in
/// tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{force_with_mass, EntityId, ForceVector, Mass};
/// let f = force_with_mass(EntityId::new(1), ForceVector::new(10.0, 0.0, 0.0), Mass::new(5.0));
/// assert_eq!(f.entity, 1);
/// assert_eq!(f.mass.unwrap().into_inner(), 5.0);
/// ```
pub fn force_with_mass(entity: EntityId, vec: ForceVector, mass: Mass) -> Force {
    force_inner(entity, vec, Some(mass))
}

/// Convenience constructor for [`BlockSlope`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{slope, BlockId, Gradient};
/// let s = slope(BlockId::new(1), Gradient::new(1.0, 0.5));
/// assert_eq!(s.block_id, 1);
/// assert_eq!(s.grad_x.into_inner(), 1.0);
/// assert_eq!(s.grad_y.into_inner(), 0.5);
/// ```
pub fn slope(block_id: BlockId, gradient: Gradient) -> BlockSlope {
    BlockSlope {
        block_id: block_id.0,
        grad_x: gradient.x.into(),
        grad_y: gradient.y.into(),
    }
}

/// Convenience constructor for [`Block`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::{block, BlockId, BlockCoords};
/// let b = block(BlockId::new(1), BlockCoords::new(0, 0, 0));
/// assert_eq!(b.id, 1);
/// assert_eq!(b.x, 0);
/// assert_eq!(b.y, 0);
/// assert_eq!(b.z, 0);
/// ```
pub fn block(id: BlockId, coords: BlockCoords) -> Block {
    Block {
        id: id.0,
        x: coords.x,
        y: coords.y,
        z: coords.z,
    }
}
