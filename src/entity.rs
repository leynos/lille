use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct Entity {
    pub position: (f32, f32, f32),
}

pub trait CausesFear {
    fn meanness_factor(&self) -> f32;
}

pub struct BadGuy {
    pub entity: Entity,
    pub meanness: f32,
}

impl CausesFear for BadGuy {
    fn meanness_factor(&self) -> f32 {
        self.meanness
    }
}