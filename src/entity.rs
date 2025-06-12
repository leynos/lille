use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Entity {
    pub position: Vec3,
}

#[derive(Clone, Debug)]
pub struct BadGuy {
    pub entity: Entity,
    pub meanness: f32,
}
