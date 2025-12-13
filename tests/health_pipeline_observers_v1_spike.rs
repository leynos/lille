//! Behavioural tests for observer-driven DBSP damage ingress.
//!
//! These tests exercise the feature-gated Observers V1 spike that routes damage
//! events through `World::trigger` / `Commands::trigger` rather than requiring
//! direct access to `DamageInbox`.

#![cfg(feature = "observers-v1-spike")]

use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::DbspDamageIngress;
use lille::{DbspPlugin, DdlogId, Health};
use rstest::{fixture, rstest};

#[derive(Resource, Default)]
struct PendingDamage {
    events: Vec<DamageEvent>,
}

fn trigger_pending_damage(mut commands: Commands, mut pending: ResMut<PendingDamage>) {
    for event in pending.events.drain(..) {
        commands.trigger(DbspDamageIngress::from(event));
    }
}

#[fixture]
fn app_with_entity() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(DbspPlugin);
    let entity = app
        .world_mut()
        .spawn((
            DdlogId(1),
            // DBSP sync reads and writes `Transform`; include it so the test entity
            // matches the production shape.
            Transform::default(),
            Health {
                current: 90,
                max: 100,
            },
        ))
        .id();
    // Run one update cycle to finish plugin initialization and ensure all DBSP
    // sync resources/observers are installed before triggering ingress events.
    app.update();
    (app, entity)
}

const fn sequenced_damage_tick_one() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 30,
        source: DamageSource::External,
        at_tick: 1,
        seq: Some(42),
    }
}

#[rstest]
#[case::single_event(vec![sequenced_damage_tick_one()], 60)]
#[case::duplicate_events_deduplicated(
    vec![sequenced_damage_tick_one(), sequenced_damage_tick_one()],
    60,
)]
fn observer_damage_ingress_applies_expected_health(
    app_with_entity: (App, Entity),
    #[case] events: Vec<DamageEvent>,
    #[case] expected_health: u16,
) {
    let (mut app, entity) = app_with_entity;
    app.insert_resource(PendingDamage { events });
    app.add_systems(Update, trigger_pending_damage);
    app.update();

    // Health starts at 90. A 30-point damage event is ingested via the observer
    // route and applies once (deduplication), so 90 - 30 = 60.
    let health = app.world().get::<Health>(entity).expect("missing Health");
    assert_eq!(health.current, expected_health);
}
