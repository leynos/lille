//! ECS state mirror for the DBSP circuit.
//!
//! `WorldHandle` caches components from Bevy so tests and legacy systems can
//! inspect them without querying the ECS. It no longer performs physics or AI
//! simulation; the [`DbspCircuit`](crate::dbsp_circuit::DbspCircuit) is now the
//! sole authority on world logic.

use bevy::prelude::*;
use hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Block, BlockSlope, UnitType};

/// Simplified entity state synchronised with the dataflow engine.
#[derive(Clone, Serialize)]
pub struct DdlogEntity {
    /// World-space position of the entity.
    pub position: Vec3,
    /// The unit archetype determining behaviour.
    pub unit: UnitType,
    /// Current health points.
    pub health_current: u16,
    /// Maximum health points.
    pub health_max: u16,
    /// Optional point the entity attempts to reach.
    pub target: Option<Vec2>,
}

impl Default for DdlogEntity {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            unit: UnitType::Civvy { fraidiness: 0.0 },
            health_current: 0,
            health_max: 0,
            target: None,
        }
    }
}

#[derive(Resource, Default)]
/// Snapshot of ECS data mirrored for DBSP.
pub struct WorldHandle {
    /// Blocks forming the terrain grid.
    pub(crate) blocks: Vec<Block>,
    /// Optional slopes associated with blocks.
    pub(crate) slopes: HashMap<i64, BlockSlope>,
    /// Active entities indexed by identifier.
    pub(crate) entities: HashMap<i64, DdlogEntity>,
}

/// Inserts an empty [`WorldHandle`] resource.
pub fn init_world_handle_system(mut commands: Commands) {
    commands.insert_resource(WorldHandle::default());
    info!("World handle created");
}
