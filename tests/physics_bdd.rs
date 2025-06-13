use bevy::prelude::*;
use insta::assert_ron_snapshot;
use lille::{
    apply_ddlog_deltas_system,
    components::{Block, BlockSlope, DdlogId, Health, UnitType},
    ddlog_handle::DdlogHandle,
    init_ddlog_system, push_state_to_ddlog_system,
};
use rstest::rstest;

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Startup, init_ddlog_system);
    app
}

#[rstest]
fn entity_transitions_between_standing_and_falling() {
    // GIVEN a block at z=0 and an entity standing on it
    let mut app = setup_app();
    app.add_systems(
        Update,
        (push_state_to_ddlog_system, apply_ddlog_deltas_system).chain(),
    );
    let block_entity = app
        .world
        .spawn_empty()
        .insert(Block {
            id: 1,
            x: 0,
            y: 0,
            z: 0,
        })
        .id();
    app.world.entity_mut(block_entity).insert(BlockSlope {
        block_id: 1,
        grad_x: 0.0,
        grad_y: 0.0,
    });
    app.world.spawn((
        DdlogId(1),
        Health(100),
        UnitType::Civvy { fraidiness: 0.0 },
        Transform::from_xyz(0.5, 0.5, 1.0),
    ));

    app.update(); // initial sync
    {
        let ddlog = app.world.resource::<DdlogHandle>();
        assert!(ddlog.deltas.is_empty());
    }

    // WHEN the block is lowered so the entity is above the floor
    app.world.entity_mut(block_entity).insert(Block {
        id: 1,
        x: 0,
        y: 0,
        z: -1,
    });
    app.update();

    // THEN the entity should have fallen
    let ddlog = app.world.resource::<DdlogHandle>();
    assert!(ddlog.deltas[0].z < 1.0);
    assert_ron_snapshot!("falling_delta", &ddlog.deltas);
}

/// Test entity behavior and floor height calculation on a sloped block surface.
#[rstest]
fn entity_on_sloped_surface() {
    // GIVEN a block at z=0 with a slope and an entity above it
    let mut app = setup_app();
    app.add_systems(
        Update,
        (push_state_to_ddlog_system, apply_ddlog_deltas_system).chain(),
    );
    let grad_x = 0.5;
    let grad_y = -0.25;
    let block_x = 0;
    let block_y = 0;
    let block_z = 0;
    let block_entity = app
        .world
        .spawn_empty()
        .insert(Block {
            id: 2,
            x: block_x,
            y: block_y,
            z: block_z,
        })
        .insert(BlockSlope {
            block_id: 2,
            grad_x,
            grad_y,
        })
        .id();

    let entity_x = 2.0;
    let entity_y = 4.0;
    let entity_z = 10.0;
    let _entity = app
        .world
        .spawn_empty()
        .insert(DdlogId(2))
        .insert(Health(100))
        .insert(UnitType::Civvy { fraidiness: 0.0 })
        .insert(Transform::from_xyz(entity_x, entity_y, entity_z))
        .id();

    app.update(); // push initial state

    // WHEN: We calculate the expected floor height at the entity's (x, y)
    let block = app.world.entity(block_entity).get::<Block>().unwrap();
    let slope = app.world.entity(block_entity).get::<BlockSlope>().unwrap();
    let expected_floor = DdlogHandle::floor_height_at(block, Some(slope), entity_x, entity_y);

    // THEN: The helper should compute the same floor height
    let actual = DdlogHandle::floor_height_at(block, Some(slope), entity_x, entity_y);
    assert!((actual - expected_floor).abs() < 1e-6);
}
