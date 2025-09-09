//! Shared constructors for physics records and a helper to initialise a
//! `DbspCircuit` for tests and examples.

use crate::components::{Block, BlockSlope};
use crate::dbsp_circuit::{DbspCircuit, Force, Position, Velocity};
pub use test_utils::physics::{
    BlockCoords, BlockId, Coords3D, EntityId, ForceVector, Gradient, Mass,
};

/// Builds a new [`DbspCircuit`] for tests.
pub fn new_circuit() -> DbspCircuit {
    DbspCircuit::new().expect("failed to build DBSP circuit")
}

/// Constructs a [`Block`] with the given identifier and coordinates.
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


/// Constructs a [`BlockSlope`] describing the block gradient.
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


/// Builds a [`Position`] from an entity identifier and coordinates.
pub fn pos<E, C>(entity: E, coords: C) -> Position
where
    E: Into<EntityId>,
    C: Into<Coords3D>,
{
    let entity: EntityId = entity.into();
    let coords: Coords3D = coords.into();
    Position {
        entity: entity.0,
        x: coords.x.into(),
        y: coords.y.into(),
        z: coords.z.into(),
    }
}


/// Builds a [`Velocity`] with the given entity and components.
pub fn vel<E, V>(entity: E, velocity: V) -> Velocity
where
    E: Into<EntityId>,
    V: Into<Coords3D>,
{
    let entity: EntityId = entity.into();
    let velocity: Coords3D = velocity.into();
    Velocity {
        entity: entity.0,
        vx: velocity.x.into(),
        vy: velocity.y.into(),
        vz: velocity.z.into(),
    }
}


/// Constructs a [`Force`] without specifying mass.
pub fn force<E, V>(entity: E, vec: V) -> Force
where
    E: Into<EntityId>,
    V: Into<ForceVector>,
{
    let entity: EntityId = entity.into();
    let vec: ForceVector = vec.into();
    Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: None,
    }
}


/// Constructs a [`Force`] with an explicit mass.
pub fn force_with_mass<E, V, M>(entity: E, vec: V, mass: M) -> Force
where
    E: Into<EntityId>,
    V: Into<ForceVector>,
    M: Into<Mass>,
{
    let entity: EntityId = entity.into();
    let vec: ForceVector = vec.into();
    let mass: Mass = mass.into();
    Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: Some(mass.0.into()),
    }
}

