//! Tests for incremental maintenance of the entity ID mapping.

use bevy::prelude::*;

use lille::components::DdlogId;
use lille::dbsp_sync::{cache_state_for_dbsp_system, init_dbsp_system, DbspState};

#[test]
fn removes_entity_from_id_map_when_ddlog_id_removed() {
    let mut app = App::new();
    init_dbsp_system(&mut app.world).expect("failed to initialise DbspState");
    app.add_systems(Update, cache_state_for_dbsp_system);

    let entity = app.world.spawn((DdlogId(1), Transform::default())).id();

    app.update();
    {
        let state = app.world.non_send_resource::<DbspState>();
        assert_eq!(state.entity_for_id(1), Some(entity));
    }

    app.world.entity_mut(entity).remove::<DdlogId>();
    app.update();

    let state = app.world.non_send_resource::<DbspState>();
    assert!(state.entity_for_id(1).is_none());
}

#[test]
fn updates_id_map_when_ddlog_id_changed() {
    let mut app = App::new();
    init_dbsp_system(&mut app.world).expect("failed to initialise DbspState");
    app.add_systems(Update, cache_state_for_dbsp_system);

    let entity = app.world.spawn((DdlogId(1), Transform::default())).id();

    app.update();

    app.world.entity_mut(entity).insert(DdlogId(2));

    app.update();

    let state = app.world.non_send_resource::<DbspState>();
    assert!(state.entity_for_id(1).is_none());
    assert_eq!(state.entity_for_id(2), Some(entity));
}
