//! Basic entity representations used within the game.
//! Contains lightweight structs for generic entities and adversaries.
use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Entity {
    pub position: Vec3,
}

#[derive(Clone, Debug)]
pub struct BadGuy {
    /// The baddie's world position.
    pub position: Vec3,
    pub meanness: f32,
}
