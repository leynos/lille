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
#[derive(Resource)]
pub struct DdlogHandle {
    pub entities: HashMap<i64, DdlogEntity>,
}

impl Default for DdlogHandle {
    fn default() -> Self {
        // Pre-allocate space for a handful of entities to avoid rehashing
        Self {
            entities: HashMap::with_capacity(32),
        }
    }
}

/// Startup system that inserts the `DdlogHandle` resource.
/// In later phases this will initialise the real DDlog program.
pub fn init_ddlog_system(mut commands: Commands) {
    commands.insert_resource(DdlogHandle::default());
    info!("DDlog handle created");
}

impl DdlogHandle {
    /// Updates internal entity positions based on the declarative movement rules.
    pub fn infer_movement(&mut self) {
        // Collect baddie information up front to avoid borrow conflicts
        let baddies: Vec<(i64, Vec2, f32)> = self
            .entities
            .iter()
            .filter_map(|(&id, e)| match e.unit {
                UnitType::Baddie { meanness } => Some((id, e.position, meanness)),
                _ => None,
            })
            .collect();

        // Compute new positions without mutating the map during iteration
        let mut updates = Vec::with_capacity(self.entities.len());

        for (&id, entity) in self.entities.iter() {
            if let UnitType::Civvy { fraidiness } = entity.unit {
                let mut min_d2 = f32::INFINITY;
                let mut closest = None;
                let mut total_fear = 0.0;

                for &(bid, b_pos, meanness) in &baddies {
                    if bid == id {
                        continue;
                    }
                    let to_actor = entity.position - b_pos;
                    let d2 = to_actor.length_squared();
                    let fear_radius = fraidiness * meanness * 2.0;
                    if d2 < fear_radius * fear_radius {
                        total_fear += 1.0 / (d2 + 0.001);
                    }
                    if d2 < min_d2 {
                        min_d2 = d2;
                        closest = Some(b_pos);
                    }
                }

                let mut dx = 0.0;
                let mut dy = 0.0;
                if total_fear > 0.2 {
                    if let Some(b_pos) = closest {
                        dx = (entity.position.x - b_pos.x).signum();
                        dy = (entity.position.y - b_pos.y).signum();
                    }
                } else if let Some(target) = entity.target {
                    dx = (target.x - entity.position.x).signum();
                    dy = (target.y - entity.position.y).signum();
                }

                updates.push((id, entity.position + Vec2::new(dx, dy)));
            }
        }

        for (id, new_pos) in updates {
            if let Some(ent) = self.entities.get_mut(&id) {
                ent.position = new_pos;
            }
        }
    }
}
