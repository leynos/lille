//! Helper types and logic for interfacing with `DDlog`.
//! Provides the `DdlogHandle` resource and entity representations used by syncing systems.
use bevy::prelude::*;
use hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Block, BlockSlope, UnitType};
use crate::{
    AIR_FRICTION, DEFAULT_MASS, DELTA_TIME, GRACE_DISTANCE, GRAVITY_PULL, GROUND_FRICTION,
    TERMINAL_VELOCITY,
};

const GRACE_DISTANCE_F32: f32 = GRACE_DISTANCE as f32;
const GRAVITY_PULL_F32: f32 = GRAVITY_PULL as f32;
const DELTA_TIME_F32: f32 = DELTA_TIME as f32;
const TERMINAL_VELOCITY_F32: f32 = TERMINAL_VELOCITY as f32;
const GROUND_FRICTION_F32: f32 = GROUND_FRICTION as f32;
const AIR_FRICTION_F32: f32 = AIR_FRICTION as f32;
const DEFAULT_MASS_F32: f32 = DEFAULT_MASS as f32;

#[derive(Clone, Serialize)]
pub struct DdlogEntity {
    pub position: Vec3,
    pub velocity: Vec3,
    pub unit: UnitType,
    pub health: i32,
    pub target: Option<Vec2>,
}

impl Default for DdlogEntity {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
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

#[derive(Clone, Serialize)]
pub struct NewVelocity {
    pub entity: i64,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
}

#[derive(Resource, Default)]
pub struct DdlogHandle {
    pub blocks: Vec<Block>,
    pub slopes: HashMap<i64, BlockSlope>,
    pub entities: HashMap<i64, DdlogEntity>,
    pub deltas: Vec<NewPosition>,
    pub velocity_deltas: Vec<NewVelocity>,
    pub forces: HashMap<i64, Vec3>,
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

    /// Apply a force to an entity for the next tick.
    pub fn apply_force(&mut self, id: i64, force: Vec3) {
        self.forces.insert(id, force);
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

    fn compute_entity_update(&self, id: i64, ent: &DdlogEntity) -> (Vec3, Vec3) {
        let floor = self.floor_height_at_point(ent.position.x, ent.position.y);
        let unsupported = ent.position.z > floor + GRACE_DISTANCE_F32;

        let mut acceleration = Vec3::ZERO;
        if let Some(force) = self.forces.get(&id) {
            acceleration += *force / DEFAULT_MASS_F32;
        }

        if unsupported {
            acceleration.z += GRAVITY_PULL_F32;
        }

        let vel_xy = ent.velocity.truncate();
        let h_mag = vel_xy.length();
        if h_mag > 0.0 {
            let coeff = if unsupported {
                AIR_FRICTION_F32
            } else {
                GROUND_FRICTION_F32
            };
            let decel_mag = h_mag.min(coeff);
            let dir = vel_xy / h_mag;
            acceleration.x -= dir.x * decel_mag;
            acceleration.y -= dir.y * decel_mag;
        }

        let mut new_vel = ent.velocity + acceleration * DELTA_TIME_F32;
        if unsupported {
            new_vel.z = new_vel
                .z
                .clamp(-TERMINAL_VELOCITY_F32, TERMINAL_VELOCITY_F32);
        } else {
            new_vel.z = 0.0;
        }

        let walk = self.civvy_move(id, ent, ent.position);
        let mut pos = ent.position;
        if unsupported {
            pos += new_vel;
        } else {
            pos += Vec3::new(new_vel.x + walk.x, new_vel.y + walk.y, 0.0);
            pos.z = floor;
        }

        (pos, new_vel)
    }

    pub fn step(&mut self) {
        let updates: Vec<(i64, Vec3, Vec3)> = self
            .entities
            .iter()
            .map(|(&id, ent)| {
                let (pos, vel) = self.compute_entity_update(id, ent);
                (id, pos, vel)
            })
            .collect();

        self.deltas.clear();
        self.velocity_deltas.clear();
        for (id, pos, vel) in updates {
            if let Some(ent) = self.entities.get_mut(&id) {
                if pos != ent.position {
                    ent.position = pos;
                    self.deltas.push(NewPosition {
                        entity: id,
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    });
                } else {
                    ent.position = pos;
                }
                if vel != ent.velocity {
                    ent.velocity = vel;
                    self.velocity_deltas.push(NewVelocity {
                        entity: id,
                        vx: vel.x,
                        vy: vel.y,
                        vz: vel.z,
                    });
                } else {
                    ent.velocity = vel;
                }
            }
        }
        self.forces.clear();
    }
}
