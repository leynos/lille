pub mod actor;
pub mod entity;
pub mod graphics;
pub mod world;
pub mod logging;

// Re-export commonly used items
pub use actor::Actor;
pub use entity::{Entity, BadGuy, CausesFear};
pub use graphics::{GraphicsContext, WINDOW_SIZE, PIXEL_SIZE};
pub use world::GameWorld;
pub use logging::init as init_logging;