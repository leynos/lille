//! Helper types and logic for interfacing with `DDlog`.
//! Provides the `DdlogHandle` resource and entity representations used by syncing systems.
use bevy::prelude::*;
use hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Block, BlockSlope, UnitType};
use crate::{GRACE_DISTANCE, GRAVITY_PULL};

#[cfg(feature = "ddlog")]
use lille_ddlog::api::{self, DDValue, HDDlog, Update};

const GRACE_DISTANCE_F32: f32 = GRACE_DISTANCE as f32;
const GRAVITY_PULL_F32: f32 = GRAVITY_PULL as f32;

#[derive(Clone, Serialize)]
pub struct DdlogEntity {
    pub position: Vec3,
    pub unit: UnitType,
    pub health: i32,
    pub target: Option<Vec2>,
}

impl Default for DdlogEntity {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            unit: UnitType::Civvy { fraidiness: 0.0 },
            health: 0,
            target: None,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct NewPosition {
    pub entity: i64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Resource)]
pub struct DdlogHandle {
    #[cfg(feature = "ddlog")]
    pub prog: Option<HDDlog>,
    pub blocks: Vec<Block>,
    pub slopes: HashMap<i64, BlockSlope>,
    pub entities: HashMap<i64, DdlogEntity>,
    pub deltas: Vec<NewPosition>,
}

impl Default for DdlogHandle {
    fn default() -> Self {
        #[cfg(feature = "ddlog")]
        let prog = match api::run(1, false) {
            Ok((p, _)) => Some(p),
            Err(e) => {
                log::error!("failed to start DDlog: {e}");
                None
            }
        };
        Self {
            #[cfg(feature = "ddlog")]
            prog,
            blocks: Vec::new(),
            slopes: HashMap::new(),
            entities: HashMap::new(),
            deltas: Vec::new(),
        }
    }
}

pub fn init_ddlog_system(mut commands: Commands) {
    commands.insert_resource(DdlogHandle::default());
    info!("DDlog handle created");
}

impl DdlogHandle {
    fn highest_block_at(&self, x: i32, y: i32) -> Option<&Block> {
        self.blocks
            .iter()
            .filter(|b| b.x == x && b.y == y)
            .max_by_key(|b| b.z)
    }

    /// Calculates the floor height at `(x, y)` relative to a block.
    pub fn floor_height_at(block: &Block, slope: Option<&BlockSlope>, x: f32, y: f32) -> f32 {
        let base = block.z as f32 + 1.0;
        if let Some(s) = slope {
            base + (x - block.x as f32) * s.grad_x + (y - block.y as f32) * s.grad_y
        } else {
            base
        }
    }

    fn floor_height_at_point(&self, x: f32, y: f32) -> f32 {
        let x_grid = x.floor() as i32;
        let y_grid = y.floor() as i32;
        if let Some(block) = self.highest_block_at(x_grid, y_grid) {
            let slope = self.slopes.get(&block.id);
            DdlogHandle::floor_height_at(block, slope, x, y)
        } else {
            0.0
        }
    }

    fn apply_gravity(&self, pos: &mut Vec3, floor: f32) {
        if pos.z > floor + GRACE_DISTANCE_F32 {
            pos.z += GRAVITY_PULL_F32;
        } else {
            pos.z = floor;
        }
    }
    fn civvy_move(&self, id: i64, ent: &DdlogEntity, pos: Vec3) -> Vec2 {
        let fraidiness = match ent.unit {
            UnitType::Civvy { fraidiness } => fraidiness,
            _ => return Vec2::ZERO,
        };

        let mut min_d2 = f32::INFINITY;
        let mut closest = None;
        let mut total_fear = 0.0;

        for (&bid, b_ent) in self.entities.iter() {
            if let UnitType::Baddie { meanness } = b_ent.unit {
                if bid == id {
                    continue;
                }
                let to_actor = pos.truncate() - b_ent.position.truncate();
                let d2 = to_actor.length_squared();
                let fear_radius = fraidiness * meanness * 2.0;
                if d2 < fear_radius * fear_radius {
                    total_fear += 1.0 / (d2 + 0.001);
                }
                if d2 < min_d2 {
                    min_d2 = d2;
                    closest = Some(b_ent.position);
                }
            }
        }

        if total_fear > 0.2 {
            if let Some(b_pos) = closest {
                return Vec2::new((pos.x - b_pos.x).signum(), (pos.y - b_pos.y).signum());
            }
        } else if let Some(target) = ent.target {
            return Vec2::new((target.x - pos.x).signum(), (target.y - pos.y).signum());
        }

        Vec2::ZERO
    }

    fn compute_entity_update(&self, id: i64, ent: &DdlogEntity) -> Vec3 {
        let floor = self.floor_height_at_point(ent.position.x, ent.position.y);
        let mut pos = ent.position;
        self.apply_gravity(&mut pos, floor);
        let delta = self.civvy_move(id, ent, pos);
        pos.x += delta.x;
        pos.y += delta.y;
        pos
    }

    pub fn step(&mut self) {
        #[cfg(feature = "ddlog")]
        if let Some(prog) = &self.prog {
            use lille_ddlog::Relations;

            let mut upds = Vec::new();
            for (&id, ent) in self.entities.iter() {
                match DDValue::from(&(id, ent)) {
                    Ok(val) => upds.push(Update {
                        relid: Relations::Position as usize,
                        weight: 1,
                        value: val,
                    }),
                    Err(e) => {
                        log::error!("failed to encode entity {id}: {e}");
                    }
                }
            }

            if let Err(e) = prog.transaction_start() {
                log::error!("DDlog transaction_start failed: {e}");
            } else if let Err(e) = prog.apply_updates(&mut upds.into_iter()) {
                log::error!("DDlog apply_updates failed: {e}");
            } else if let Err(e) = prog.transaction_commit_dump_changes() {
                log::error!("DDlog commit failed: {e}");
            }
        }

        let updates: Vec<(i64, Vec3)> = self
            .entities
            .iter()
            .map(|(&id, ent)| (id, self.compute_entity_update(id, ent)))
            .collect();

        self.deltas.clear();
        for (id, pos) in updates {
            if let Some(ent) = self.entities.get_mut(&id) {
                if pos != ent.position {
                    ent.position = pos;
                    self.deltas.push(NewPosition {
                        entity: id,
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    });
                }
            }
        }
    }
}
