//! Regressions guarding Bevy 0.14 scheduling changes.

mod common;

use common::{DbspAssertions, DbspTestAppBuilder};
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::DbspState;
use lille::DamageInbox;
use rstest::rstest;

#[rstest]
fn non_send_state_survives_updates() {
    let mut app = DbspTestAppBuilder::new().prime().build();

    let initial_ptr = {
        let world = app.world();
        let state = world
            .get_non_send_resource::<DbspState>()
            .expect("DbspState should be initialised by DbspPlugin");
        std::ptr::from_ref(state)
    };

    for _ in 0..5 {
        app.update();
    }

    let after_ptr = {
        let world = app.world();
        let state = world
            .get_non_send_resource::<DbspState>()
            .expect("DbspState should remain after running multiple Update ticks");
        std::ptr::from_ref(state)
    };

    assert_eq!(
        initial_ptr, after_ptr,
        "DbspState should remain stable across frames"
    );
}

#[rstest]
fn damage_events_apply_in_a_single_frame() {
    let (builder, entity) = DbspTestAppBuilder::new().spawn_entity_with_health(1, 100, 100);
    let (mut app, tracked_entity) = builder.build_with_entity(entity);

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

    DbspAssertions::assert_health_current(&app, tracked_entity, 75)
        .expect("Health should be 75 after damage");
    DbspAssertions::assert_duplicate_count(&app, 0).expect("No duplicates should be recorded");
    DbspAssertions::assert_inbox_empty(&app).expect("DamageInbox should be drained");
}
