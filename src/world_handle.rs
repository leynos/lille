//! In-memory world state manager used by the physics systems.
//!
//! This handle stores entities and blocks for processing each tick. It
//! implements basic gravity and movement without any external runtime.

use bevy::prelude::*;
use hashbrown::HashMap;
use serde::Serialize;

use crate::components::{Block, BlockSlope, UnitType};
use crate::{
    BLOCK_TOP_OFFSET, FEAR_DISTANCE_EPSILON, FEAR_RADIUS_MULTIPLIER, FEAR_THRESHOLD,
    GRACE_DISTANCE, GRAVITY_PULL,
};

/// Grid coordinate in the world terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoordinate {
    pub x: i32,
    pub y: i32,
}

impl From<(i32, i32)> for GridCoordinate {
    fn from((x, y): (i32, i32)) -> Self {
        Self { x, y }
    }
}

/// Entity identifier with type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct EntityId(pub i64);

impl From<i64> for EntityId {
    fn from(id: i64) -> Self {
        Self(id)
    }
}

impl EntityId {
    pub fn into_inner(self) -> i64 {
        self.0
    }
}

/// Fear level measurement for entities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FearLevel(pub f32);

impl FearLevel {
    pub fn into_inner(self) -> f32 {
        self.0
    }
}

impl std::ops::Add for FearLevel {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::ops::AddAssign for FearLevel {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

/// World position coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPosition {
    pub x: f32,
    pub y: f32,
}

impl From<(f32, f32)> for WorldPosition {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

/// Height measurement in world units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Height(pub f32);

impl Height {
    pub fn into_inner(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Serialize)]
/// Simplified entity state synchronised with the dataflow engine.
pub struct DdlogEntity {
    /// World-space position of the entity.
    pub position: Vec3,
    /// The unit archetype determining behaviour.
    pub unit: UnitType,
    /// Current health points.
    pub health: i32,
    /// Optional point the entity attempts to reach.
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
/// Discrete update describing an entity's new position.
pub struct NewPosition {
    /// Identifier of the moved entity.
    pub entity: EntityId,
    /// New x-coordinate.
    pub x: f32,
    /// New y-coordinate.
    pub y: f32,
    /// New z-coordinate.
    pub z: f32,
}

#[derive(Resource, Default)]
/// In-memory snapshot of the world used for physics simulation.
pub struct WorldHandle {
    /// Blocks forming the terrain grid.
    pub blocks: Vec<Block>,
    /// Optional slopes associated with blocks.
    pub slopes: HashMap<i64, BlockSlope>,
    /// Active entities indexed by identifier.
    pub entities: HashMap<i64, DdlogEntity>,
    /// Pending position deltas emitted after stepping.
    pub deltas: Vec<NewPosition>,
}

pub fn init_world_handle_system(mut commands: Commands) {
    commands.insert_resource(WorldHandle::default());
    info!("World handle created");
}

impl WorldHandle {
    fn highest_block_at(&self, coord: GridCoordinate) -> Option<&Block> {
        self.blocks
            .iter()
            .filter(|b| b.x == coord.x && b.y == coord.y)
            .max_by_key(|b| b.z)
    }

    pub fn floor_height_at(
        block: &Block,
        slope: Option<&BlockSlope>,
        pos: WorldPosition,
    ) -> Height {
        let base = block.z as f32 + BLOCK_TOP_OFFSET as f32;
        if let Some(s) = slope {
            Height(
                base + (pos.x - block.x as f32) * s.grad_x.into_inner() as f32
                    + (pos.y - block.y as f32) * s.grad_y.into_inner() as f32,
            )
        } else {
            Height(base)
        }
    }

    fn floor_height_at_point(&self, pos: WorldPosition) -> Height {
        let coord = GridCoordinate::from((pos.x.floor() as i32, pos.y.floor() as i32));
        if let Some(block) = self.highest_block_at(coord) {
            let slope = self.slopes.get(&block.id);
            WorldHandle::floor_height_at(block, slope, pos)
        } else {
            Height(0.0)
        }
    }

    fn apply_gravity(&self, pos: &mut Vec3, floor: Height) {
        if pos.z > floor.into_inner() + GRACE_DISTANCE as f32 {
            pos.z += GRAVITY_PULL as f32;
        } else {
            pos.z = floor.into_inner();
        }
    }

    /// Aggregate fear from nearby baddies and return the closest one.
    fn aggregate_fear(
        &self,
        id: EntityId,
        pos: Vec3,
        fraidiness: FearLevel,
    ) -> (FearLevel, Option<Vec3>) {
        let mut min_d2 = f32::INFINITY;
        let mut closest = None;
        let mut total_fear = FearLevel(0.0);

        for (&bid, b_ent) in self.entities.iter() {
            if let UnitType::Baddie { meanness } = b_ent.unit {
                if EntityId::from(bid) == id {
                    continue;
                }
                let to_actor = pos.truncate() - b_ent.position.truncate();
                let d2 = to_actor.length_squared();
                let fear_radius =
                    fraidiness.into_inner() * meanness * FEAR_RADIUS_MULTIPLIER as f32;
                if d2 < fear_radius * fear_radius {
                    // Fear increases as distance to the threat decreases.
                    total_fear += FearLevel(1.0_f32 / (d2 + FEAR_DISTANCE_EPSILON as f32));
                }
                if d2 < min_d2 {
                    min_d2 = d2;
                    closest = Some(b_ent.position);
                }
            }
        }

        (total_fear, closest)
    }

    /// Decide movement vector based on fear and the current target.
    fn movement_vector(
        fear: FearLevel,
        closest: Option<Vec3>,
        target: Option<Vec2>,
        pos: Vec3,
    ) -> Vec2 {
        if fear.into_inner() > FEAR_THRESHOLD as f32 {
            if let Some(b_pos) = closest {
                // Move away from the nearest threat when fear overwhelms.
                return Vec2::new((pos.x - b_pos.x).signum(), (pos.y - b_pos.y).signum());
            }
        } else if let Some(target) = target {
            // Advance towards the target when calm.
            return Vec2::new((target.x - pos.x).signum(), (target.y - pos.y).signum());
        }

        Vec2::ZERO
    }

    fn civvy_move(&self, id: EntityId, ent: &DdlogEntity, pos: Vec3) -> Vec2 {
        let fraidiness = match ent.unit {
            UnitType::Civvy { fraidiness } => fraidiness,
            _ => return Vec2::ZERO,
        };
        let (fear, closest) = self.aggregate_fear(id, pos, FearLevel(fraidiness));
        Self::movement_vector(fear, closest, ent.target, pos)
    }

    fn compute_entity_update(&self, id: EntityId, ent: &DdlogEntity) -> Vec3 {
        let floor =
            self.floor_height_at_point(WorldPosition::from((ent.position.x, ent.position.y)));
        let mut pos = ent.position;
        self.apply_gravity(&mut pos, floor);
        let delta = self.civvy_move(id, ent, pos);
        pos.x += delta.x;
        pos.y += delta.y;
        pos
    }

    fn collect_updates(&self) -> Vec<(EntityId, Vec3)> {
        self.entities
            .iter()
            .map(|(&id, ent)| {
                let eid = EntityId::from(id);
                (eid, self.compute_entity_update(eid, ent))
            })
            .collect()
    }

    fn apply_updates(&mut self, updates: Vec<(EntityId, Vec3)>) {
        self.deltas.clear();
        for (id, pos) in updates {
            if let Some(ent) = self.entities.get_mut(&id.into_inner()) {
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

    /// Advance the world simulation by one tick.
    ///
    /// This applies gravity and movement, recording any position changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use lille::world_handle::WorldHandle;
    /// let mut handle = WorldHandle::default();
    /// handle.step();
    /// assert!(handle.deltas.is_empty());
    /// ```
    pub fn step(&mut self) {
        let updates = self.collect_updates();
        self.apply_updates(updates);
    }
}
