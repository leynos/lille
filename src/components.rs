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

#[derive(Component, Debug, Clone, Serialize)]
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
