//! Convenience constructors for physics-related records used in tests.

mod units;

pub use units::{
    BlockCoords, BlockId, Coords2D, Coords3D, EntityId, FearValue, ForceVector, Gradient, Mass,
};

use lille::components::{Block, BlockSlope};
use lille::dbsp_circuit::{DbspCircuit, FearLevel, Force, Position, Target, Velocity};

/// Create a new [`DbspCircuit`] for tests.
///
/// # Errors
/// Returns an error when the underlying circuit cannot be constructed.
/// Fixtures propagate failures so the calling test can report the verdict.
///
/// # Examples
/// ```rust,no_run
/// use test_utils::physics::new_circuit;
/// # fn demo() -> Result<(), dbsp::Error> {
/// let circuit = new_circuit()?;
/// drop(circuit);
/// # Ok(())
/// # }
/// ```
pub fn new_circuit() -> Result<DbspCircuit, dbsp::Error> {
    DbspCircuit::new()
}

fn with_coords3<E, C, R>(entity: E, coords: C, f: impl Fn(EntityId, Coords3D) -> R) -> R
where
    E: Into<EntityId>,
    C: Into<Coords3D>,
{
    let entity: EntityId = entity.into();
    let coords: Coords3D = coords.into();
    f(entity, coords)
}

fn with_coords2<E, C, R>(entity: E, coords: C, f: impl Fn(EntityId, Coords2D) -> R) -> R
where
    E: Into<EntityId>,
    C: Into<Coords2D>,
{
    let entity: EntityId = entity.into();
    let coords: Coords2D = coords.into();
    f(entity, coords)
}

macro_rules! impl_record_constructor {
    ($(#[$attr:meta])* 3d $name:ident, $record:ident, $fx:ident, $fy:ident, $fz:ident) => {
        $(#[$attr])*
        pub fn $name<E, C>(entity: E, coords: C) -> $record
        where
            E: Into<EntityId>,
            C: Into<Coords3D>,
        {
            with_coords3(entity, coords, |entity, coords| $record {
                entity: entity.0,
                $fx: coords.x.into(),
                $fy: coords.y.into(),
                $fz: coords.z.into(),
            })
        }
    };
    ($(#[$attr:meta])* 2d $name:ident, $record:ident, $fx:ident, $fy:ident) => {
        $(#[$attr])*
        pub fn $name<E, C>(entity: E, coords: C) -> $record
        where
            E: Into<EntityId>,
            C: Into<Coords2D>,
        {
            with_coords2(entity, coords, |entity, coords| $record {
                entity: entity.0,
                $fx: coords.x.into(),
                $fy: coords.y.into(),
            })
        }
    };
}

impl_record_constructor!(
    /// Convenience constructor for [`Position`] records used in tests.
    ///
    /// # Examples
    /// ```rust
    /// use test_utils::physics::pos;
    /// let p = pos(1, (0.0, 1.0, 2.0));
    /// assert_eq!(p.entity, 1);
    /// assert_eq!(p.x.into_inner(), 0.0);
    /// assert_eq!(p.y.into_inner(), 1.0);
    /// assert_eq!(p.z.into_inner(), 2.0);
    /// ```
    3d pos, Position, x, y, z
);

impl_record_constructor!(
    /// Convenience constructor for [`Velocity`] records used in tests.
    ///
    /// # Examples
    /// ```rust
    /// use test_utils::physics::vel;
    /// let v = vel(1, (0.5, -0.5, 1.0));
    /// assert_eq!(v.entity, 1);
    /// assert_eq!(v.vx.into_inner(), 0.5);
    /// assert_eq!(v.vy.into_inner(), -0.5);
    /// assert_eq!(v.vz.into_inner(), 1.0);
    /// ```
    3d vel, Velocity, vx, vy, vz
);

impl_record_constructor!(
    /// Convenience constructor for [`Target`] records used in tests.
    ///
    /// # Examples
    /// ```rust
    /// use test_utils::physics::target;
    /// let t = target(1, (1.0, 2.0));
    /// assert_eq!(t.entity, 1);
    /// assert_eq!(t.x.into_inner(), 1.0);
    /// assert_eq!(t.y.into_inner(), 2.0);
    /// ```
    #[inline]
    2d target, Target, x, y
);

/// Convenience constructor for [`FearLevel`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::fear;
/// let f = fear(1, 0.5);
/// assert_eq!(f.entity, 1);
/// assert_eq!(f.level.into_inner(), 0.5);
/// ```
#[inline]
pub fn fear<E, L>(entity: E, level: L) -> FearLevel
where
    E: Into<EntityId>,
    L: Into<FearValue>,
{
    let entity: EntityId = entity.into();
    let level: FearValue = level.into();
    FearLevel {
        entity: entity.0,
        level: level.0.into(),
    }
}

/// Convenience constructor for [`Force`] records without mass used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::force;
/// let f = force(1, (10.0, 0.0, 0.0));
/// assert_eq!(f.entity, 1);
/// assert!(f.mass.is_none());
/// ```
fn force_inner(entity: EntityId, vec: ForceVector, mass: Option<Mass>) -> Force {
    Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: mass.map(|m| m.into()),
    }
}

pub fn force<E, V>(entity: E, vec: V) -> Force
where
    E: Into<EntityId>,
    V: Into<ForceVector>,
{
    let entity: EntityId = entity.into();
    let vec: ForceVector = vec.into();
    force_inner(entity, vec, None)
}

/// Convenience constructor for [`Force`] records with an explicit mass used in
/// tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::force_with_mass;
/// let f = force_with_mass(1, (10.0, 0.0, 0.0), 5.0);
/// assert_eq!(f.entity, 1);
/// assert_eq!(f.mass.unwrap().into_inner(), 5.0);
/// ```
pub fn force_with_mass<E, V, M>(entity: E, vec: V, mass: M) -> Force
where
    E: Into<EntityId>,
    V: Into<ForceVector>,
    M: Into<Mass>,
{
    let entity: EntityId = entity.into();
    let vec: ForceVector = vec.into();
    let mass: Mass = mass.into();
    force_inner(entity, vec, Some(mass))
}

/// Convenience constructor for [`BlockSlope`] records used in tests.
///
/// # Examples
/// ```rust
/// use test_utils::physics::slope;
/// let s = slope(1, (1.0, 0.5));
/// assert_eq!(s.block_id, 1);
/// assert_eq!(s.grad_x.into_inner(), 1.0);
/// assert_eq!(s.grad_y.into_inner(), 0.5);
/// ```
pub fn slope<I, G>(block_id: I, gradient: G) -> BlockSlope
where
    I: Into<BlockId>,
    G: Into<Gradient>,
{
    let block_id: BlockId = block_id.into();
    let gradient: Gradient = gradient.into();
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
/// use test_utils::physics::block;
/// let b = block(1, (0, 0, 0));
/// assert_eq!(b.id, 1);
/// assert_eq!(b.x, 0);
/// assert_eq!(b.y, 0);
/// assert_eq!(b.z, 0);
/// ```
pub fn block<I, C>(id: I, coords: C) -> Block
where
    I: Into<BlockId>,
    C: Into<BlockCoords>,
{
    let id: BlockId = id.into();
    let coords: BlockCoords = coords.into();
    Block {
        id: id.0,
        x: coords.x,
        y: coords.y,
        z: coords.z,
    }
}
