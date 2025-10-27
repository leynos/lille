//! Basic entity representations used within the game.
//! Contains lightweight structs for generic entities and adversaries.
use glam::Vec3;

/// Minimal representation of any world entity tracked outside Bevy.
#[derive(Clone, Debug)]
pub struct WorldEntity {
    /// World position used by AI and DBSP synchronisation.
    pub position: Vec3,
}

/// Specialised entity describing hostile actors.
#[derive(Clone, Debug)]
pub struct BadGuy {
    /// The baddie's world position.
    pub position: Vec3,
    /// Aggression factor that influences combat reactions.
    pub meanness: f32,
}
