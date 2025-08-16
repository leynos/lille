//! Tests for incremental maintenance of the entity ID mapping.

use bevy::prelude::*;

use lille::components::DdlogId;
use lille::dbsp_sync::{cache_state_for_dbsp_system, init_dbsp_system, DbspState};

#[test]
fn removes_entity_from_id_map_when_ddlog_id_removed() {
    let mut app = App::new();
    init_dbsp_system(&mut app.world).unwrap();
    app.add_systems(Update, cache_state_for_dbsp_system);

    let entity = app.world.spawn((DdlogId(1), Transform::default())).id();

    app.update();
    {
        let state = app.world.non_send_resource::<DbspState>();
        assert_eq!(state.entity_for_id(1), Some(entity));
    }

    app.world.despawn(entity);
    app.update();

    let state = app.world.non_send_resource::<DbspState>();
    assert!(state.entity_for_id(1).is_none());
}
