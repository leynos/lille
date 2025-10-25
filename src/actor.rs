//! Actor data structures and related logic.
//! Defines the `Actor` component and behaviour utilities for movement systems.
use crate::entity::WorldEntity;
use glam::Vec3;

/// Data representation of an in-game actor.
#[derive(Clone, Debug)]
pub struct Actor {
    /// World entity representing this actor in the ECS world.
    pub entity: WorldEntity,
    /// Desired world-space destination for steering logic.
    pub target: Vec3,
    /// Movement speed in metres per second.
    pub speed: f32,
    /// How easily this actor becomes scared.
    pub fraidiness: f32,
}
