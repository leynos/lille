//! ECS component types used by the game.
//! Includes identifiers, health, target positions, and unit descriptors shared between systems.
use bevy::prelude::*;
use serde::Serialize;

#[derive(Component, Debug, Serialize)]
pub struct DdlogId(pub i64);

#[derive(Component, Default, Serialize)]
pub struct Health(pub i32);

#[derive(Component, Debug, Clone, Serialize)]
pub enum UnitType {
    Civvy { fraidiness: f32 },
    Baddie { meanness: f32 },
}

#[derive(Component, Debug, Deref, DerefMut, Serialize)]
pub struct Target(pub Vec2);

use rkyv::{Archive, Deserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

#[derive(
    Archive,
    RkyvSerialize,
    Deserialize,
    Component,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
pub struct Block {
    pub id: i64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Component, Debug, Clone, Serialize)]
pub struct BlockSlope {
    pub block_id: i64,
    pub grad_x: f32,
    pub grad_y: f32,
}
#[derive(Component, Debug, Clone, Default, Serialize)]
pub struct VelocityComp {
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
}
