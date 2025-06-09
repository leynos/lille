use bevy::prelude::*;
use lille::{spawn_world_system, GameWorld, UnitType};

#[test]
fn spawns_world_entities() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.world.insert_resource(GameWorld::default());
    app.add_systems(Startup, spawn_world_system);
    app.update();

    let world = &mut app.world;
    let mut civvy = 0;
    let mut baddie = 0;
    for unit in world.query::<&UnitType>().iter(world) {
        match unit {
            UnitType::Civvy { .. } => civvy += 1,
            UnitType::Baddie { .. } => baddie += 1,
        }
    }
    assert_eq!(civvy, 1);
    assert_eq!(baddie, 1);
}
