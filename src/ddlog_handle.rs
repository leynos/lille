use bevy::prelude::*;
use hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Block, BlockSlope, UnitType};

const GRACE_DISTANCE: f32 = 0.1;
const GRAVITY_PULL: f32 = -1.0;

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

#[derive(Resource, Default)]
pub struct DdlogHandle {
    pub blocks: Vec<Block>,
    pub slopes: HashMap<i64, BlockSlope>,
    pub entities: HashMap<i64, DdlogEntity>,
    pub deltas: Vec<NewPosition>,
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

    /// Calculates the floor height at `(x, y)` given a block base and slope.
    pub fn floor_height_at(
        block_x: f32,
        block_y: f32,
        block_z: f32,
        grad_x: f32,
        grad_y: f32,
        x: f32,
        y: f32,
    ) -> f32 {
        block_z + 1.0 + (x - block_x) * grad_x + (y - block_y) * grad_y
    }

    fn floor_height_at_point(&self, x: f32, y: f32) -> f32 {
        let x_grid = x.floor() as i32;
        let y_grid = y.floor() as i32;
        if let Some(block) = self.highest_block_at(x_grid, y_grid) {
            if let Some(slope) = self.slopes.get(&block.id) {
                DdlogHandle::floor_height_at(
                    block.x as f32,
                    block.y as f32,
                    block.z as f32,
                    slope.grad_x,
                    slope.grad_y,
                    x,
                    y,
                )
            } else {
                (block.z as f32) + 1.0
            }
        } else {
            0.0
        }
    }

    pub fn step(&mut self) {
        let updates: Vec<(i64, Vec3)> = self
            .entities
            .iter()
            .map(|(&id, ent)| {
                let floor = self.floor_height_at_point(ent.position.x, ent.position.y);
                let mut pos = ent.position;
                if pos.z > floor + GRACE_DISTANCE {
                    pos.z += GRAVITY_PULL;
                } else {
                    pos.z = floor;
                }
                if let UnitType::Civvy { fraidiness } = ent.unit {
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

                    let mut dx = 0.0;
                    let mut dy = 0.0;
                    if total_fear > 0.2 {
                        if let Some(b_pos) = closest {
                            dx = (pos.x - b_pos.x).signum();
                            dy = (pos.y - b_pos.y).signum();
                        }
                    } else if let Some(target) = ent.target {
                        dx = (target.x - pos.x).signum();
                        dy = (target.y - pos.y).signum();
                    }

                    pos.x += dx;
                    pos.y += dy;
                }
                (id, pos)
            })
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
