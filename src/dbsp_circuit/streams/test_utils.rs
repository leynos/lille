//! Shared constructors for physics records and a helper to initialise a
//! `DbspCircuit` for tests and examples.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::{DbspCircuit, Force, Position, Velocity};
pub use test_utils::physics::{
    BlockCoords, BlockId, Coords3D, EntityId, ForceVector, Gradient, Mass,
};

macro_rules! impl_test_helper {
    (
        $(#[$attr:meta])*
        $fn_name:ident<$($generic:ident: Into<$target:ty>),+>($($param:ident: $generic_param:ident),+) -> $ret_type:path {
            $($field:ident: $expr:expr),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        pub fn $fn_name<$($generic),+>($($param: $generic_param),+) -> $ret_type
        where
            $($generic: Into<$target>),+
        {
            $(let $param: $target = $param.into();)+
            $ret_type {
                $($field: $expr),+
            }
        }
    };
}

/// Builds a new [`DbspCircuit`] for tests.
///
/// # Examples
/// ```rust,no_run
/// use lille::dbsp_circuit::streams::test_utils::new_circuit;
///
/// # fn demo() -> Result<(), dbsp::Error> {
/// let mut circuit = new_circuit();
/// // push inputs here, e.g. circuit.position_in().push(position, 1);
/// circuit.step()?; // propagate evaluation errors with `?`
/// # Ok(())
/// # }
/// ```
///
/// Builds a new [`DbspCircuit`] configured for stream tests.
///
/// # Panics
/// Panics if the underlying [`DbspCircuit::new`] call fails to construct the circuit.
#[must_use]
pub fn new_circuit() -> DbspCircuit {
    match DbspCircuit::new() {
        Ok(circuit) => circuit,
        Err(error) => panic!("failed to build DBSP circuit: {error}"),
    }
}

/// Constructs a [`Block`] with the given identifier and coordinates.
pub fn block<I, C>(id: I, coords: C) -> Block
where
    I: Into<BlockId>,
    C: Into<BlockCoords>,
{
    let block_id: BlockId = id.into();
    let block_coords: BlockCoords = coords.into();
    Block {
        id: block_id.0,
        x: block_coords.x,
        y: block_coords.y,
        z: block_coords.z,
    }
}

/// Constructs a [`BlockSlope`] describing the block gradient.
pub fn slope<I, G>(block_id: I, gradient: G) -> BlockSlope
where
    I: Into<BlockId>,
    G: Into<Gradient>,
{
    let slope_block_id: BlockId = block_id.into();
    let slope_gradient: Gradient = gradient.into();
    BlockSlope {
        block_id: slope_block_id.0,
        grad_x: slope_gradient.x.into(),
        grad_y: slope_gradient.y.into(),
    }
}

impl_test_helper!(
    /// Builds a [`Position`] from an entity identifier and coordinates.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use lille::dbsp_circuit::streams::test_utils::{pos, EntityId, Coords3D};
    /// let p = pos(EntityId(42), Coords3D { x: 1.0, y: 2.0, z: 3.0 });
    /// assert_eq!(p.entity, 42);
    /// assert_eq!((p.x, p.y, p.z), (1.0, 2.0, 3.0));
    /// ```
    pos<E: Into<EntityId>, C: Into<Coords3D> >(entity: E, coords: C) -> Position {
        entity: entity.0,
        x: coords.x.into(),
        y: coords.y.into(),
        z: coords.z.into(),
    }
);

impl_test_helper!(
    /// Builds a [`Velocity`] with the given entity and components.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use lille::dbsp_circuit::streams::test_utils::{vel, EntityId, Coords3D};
    /// let v = vel(EntityId(42), Coords3D { x: 0.1, y: 0.0, z: -0.1 });
    /// assert_eq!(v.entity, 42);
    /// assert_eq!((v.vx, v.vy, v.vz), (0.1, 0.0, -0.1));
    /// ```
    vel<E: Into<EntityId>, V: Into<Coords3D> >(entity: E, velocity: V) -> Velocity {
        entity: entity.0,
        vx: velocity.x.into(),
        vy: velocity.y.into(),
        vz: velocity.z.into(),
    }
);

impl_test_helper!(
    /// Constructs a [`Force`] without specifying mass.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use lille::dbsp_circuit::streams::test_utils::{force, EntityId, ForceVector};
    /// let f = force(EntityId(7), ForceVector { x: 3.0, y: 0.0, z: -1.0 });
    /// assert_eq!(f.entity, 7);
    /// assert_eq!((f.fx, f.fy, f.fz), (3.0, 0.0, -1.0));
    /// assert!(f.mass.is_none());
    /// ```
    force<E: Into<EntityId>, V: Into<ForceVector> >(entity: E, vec: V) -> Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: None,
    }
);

impl_test_helper!(
    /// Constructs a [`Force`] with an explicit mass.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use ordered_float::OrderedFloat;
    /// use lille::dbsp_circuit::streams::test_utils::{
    ///     force_with_mass, EntityId, ForceVector, Mass,
    /// };
    /// let f = force_with_mass(
    ///     EntityId(7),
    ///     ForceVector { x: 3.0, y: 0.0, z: -1.0 },
    ///     Mass(2.0),
    /// );
    /// assert_eq!(f.entity, 7);
    /// assert_eq!((f.fx, f.fy, f.fz), (3.0, 0.0, -1.0));
    /// assert_eq!(f.mass, Some(OrderedFloat(2.0)));
    /// ```
    force_with_mass<E: Into<EntityId>, V: Into<ForceVector>, M: Into<Mass> >(entity: E, vec: V, mass: M) -> Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: Some(mass.into()),
    }
);
