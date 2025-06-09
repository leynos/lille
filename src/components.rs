use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct DdlogId(pub i64);

#[derive(Component, Default)]
pub struct Health(pub i32);

#[derive(Component, Debug, Clone)]
pub enum UnitType {
    Civvy { fraidiness: f32 },
    Baddie { meanness: f32 },
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct Target(pub Vec2);
