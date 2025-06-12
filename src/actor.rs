use crate::entity::Entity;
use glam::Vec3;

/// Data representation of an in-game actor.
#[derive(Clone, Debug)]
pub struct Actor {
    pub entity: Entity,
    pub target: Vec3,
    pub speed: f32,
    /// How easily this actor becomes scared.
    pub fraidiness: f32,
}
