//! Shared constructors for physics records used in tests.

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
pub fn block(id: BlockId, coords: BlockCoords) -> Block {
    Block {
        id: id.0,
        x: coords.x,
        y: coords.y,
        z: coords.z,
    }
}

/// Constructs a [`BlockSlope`] describing the block gradient.
pub fn slope(block_id: BlockId, gradient: Gradient) -> BlockSlope {
    BlockSlope {
        block_id: block_id.0,
        grad_x: gradient.x.into(),
        grad_y: gradient.y.into(),
    }
}

/// Builds a [`Position`] from an entity identifier and coordinates.
pub fn pos(entity: EntityId, coords: Coords3D) -> Position {
    Position {
        entity: entity.0,
        x: coords.x.into(),
        y: coords.y.into(),
        z: coords.z.into(),
    }
}

/// Builds a [`Velocity`] with the given entity and components.
pub fn vel(entity: EntityId, velocity: Coords3D) -> Velocity {
    Velocity {
        entity: entity.0,
        vx: velocity.x.into(),
        vy: velocity.y.into(),
        vz: velocity.z.into(),
    }
}

/// Constructs a [`Force`] without specifying mass.
pub fn force(entity: EntityId, vec: ForceVector) -> Force {
    Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: None,
    }
}

/// Constructs a [`Force`] with an explicit mass.
pub fn force_with_mass(entity: EntityId, vec: ForceVector, mass: Mass) -> Force {
    Force {
        entity: entity.0,
        fx: vec.x.into(),
        fy: vec.y.into(),
        fz: vec.z.into(),
        mass: Some(mass.0.into()),
    }
}
