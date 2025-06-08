use bevy::prelude::*;

/// Resource holding the DDlog runtime handle.
///
/// The actual DDlog runtime is not initialised in this phase.
#[derive(Resource)]
pub struct DdlogHandle;

/// Startup system that inserts the `DdlogHandle` resource.
/// In later phases this will initialise the real DDlog program.
pub fn init_ddlog_system(mut commands: Commands) {
    commands.insert_resource(DdlogHandle);
    info!("DDlog handle created");
}
