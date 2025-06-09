pub mod actor;
pub mod components;
pub mod ddlog_handle;
pub mod ddlog_sync;
pub mod entity;
pub mod graphics;
pub mod logging;
pub mod spawn_world;
pub mod world;

// Re-export commonly used items
pub use actor::Actor;
pub use components::{DdlogId, Health, Target, UnitType};
pub use ddlog_handle::{init_ddlog_system, DdlogHandle};
pub use ddlog_sync::push_state_to_ddlog_system;
pub use entity::{BadGuy, CausesFear, Entity};
pub use graphics::{GraphicsContext, PIXEL_SIZE, WINDOW_SIZE};
pub use logging::init as init_logging;
pub use spawn_world::spawn_world_system;
pub use world::GameWorld;
