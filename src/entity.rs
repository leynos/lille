use glam::Vec3;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct Entity {
    pub position: Vec3,
}

impl Entity {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, y, z),
        }
    }
}

pub trait CausesFear {
    fn meanness_factor(&self) -> f32;
}

pub struct BadGuy {
    pub entity: Entity,
    pub meanness: f32,
}

impl BadGuy {
    pub fn new(x: f32, y: f32, z: f32, meanness: f32) -> Self {
        Self {
            entity: Entity::new(x, y, z),
            meanness,
        }
    }
}

impl CausesFear for BadGuy {
    fn meanness_factor(&self) -> f32 {
        self.meanness
    }
}
