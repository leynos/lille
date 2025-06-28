//! Behaviour-driven tests for physics-related systems.
//! Uses `rstest` to script scenarios covering entity transitions.
#![cfg(not(feature = "ddlog"))]
use bevy::prelude::*;
use insta::assert_ron_snapshot;
use lille::{
    apply_ddlog_deltas_system, cache_state_for_ddlog_system,
    components::{Block, BlockSlope, DdlogId, Health, UnitType},
    ddlog_handle::DdlogHandle,
    init_ddlog_system,
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
        (cache_state_for_ddlog_system, apply_ddlog_deltas_system).chain(),
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
