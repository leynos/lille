//! Regressions guarding Bevy 0.14 scheduling changes.

use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::DbspState;
use lille::{DamageInbox, DbspPlugin, DdlogId, Health};
use rstest::rstest;

#[rstest]
fn non_send_state_survives_updates() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);

    let initial_ptr = {
        let world = app.world();
        let state = world
            .get_non_send_resource::<DbspState>()
            .expect("DbspState should be initialised by DbspPlugin");
        std::ptr::from_ref(state)
    };

    app.update();

    let after_ptr = {
        let world = app.world();
        let state = world
            .get_non_send_resource::<DbspState>()
            .expect("DbspState should remain after running the Update schedule");
        std::ptr::from_ref(state)
    };

    assert_eq!(
        initial_ptr, after_ptr,
        "DbspState should remain stable across frames"
    );
}

#[rstest]
fn damage_events_apply_in_a_single_frame() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);

    let entity = app
        .world_mut()
        .spawn((
            DdlogId(1),
            Transform::default(),
            Health {
                current: 100,
                max: 100,
            },
        ))
        .id();

    {
        let mut inbox = app.world_mut().resource_mut::<DamageInbox>();
        inbox.push(DamageEvent {
            entity: 1,
            amount: 25,
            source: DamageSource::External,
            at_tick: 1,
            seq: Some(7),
        });
    }

    app.update();

    let health = app
        .world()
        .get::<Health>(entity)
        .expect("Health should remain attached after DBSP step");
    assert_eq!(health.current, 75);

    let state = app
        .world()
        .get_non_send_resource::<DbspState>()
        .expect("DbspState should be available via get_non_send_resource");
    assert_eq!(state.applied_health_duplicates(), 0);

    let inbox = app.world().resource::<DamageInbox>();
    assert!(
        inbox.is_empty(),
        "DamageInbox should be drained in the same frame"
    );
}
