use bevy::prelude::*;
use hashbrown::HashMap;

use crate::components::UnitType;

/// Internal state for an entity tracked by the DDlog stub.
pub struct DdlogEntity {
    pub position: Vec2,
    pub unit: UnitType,
    pub health: i32,
    pub target: Option<Vec2>,
}

impl Default for DdlogEntity {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            unit: UnitType::Civvy { fraidiness: 0.0 },
            health: 0,
            target: None,
        }
    }
}

/// Resource holding the DDlog runtime handle.
///
/// The actual DDlog runtime is not initialised in this phase.
#[derive(Resource, Default)]
pub struct DdlogHandle {
    pub entities: HashMap<i64, DdlogEntity>,
}

/// Startup system that inserts the `DdlogHandle` resource.
/// In later phases this will initialise the real DDlog program.
pub fn init_ddlog_system(mut commands: Commands) {
    commands.insert_resource(DdlogHandle::default());
    info!("DDlog handle created");
}
