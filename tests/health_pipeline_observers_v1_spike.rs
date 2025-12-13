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
use rstest::rstest;

#[derive(Resource, Default)]
struct PendingDamage {
    events: Vec<DamageEvent>,
}

fn trigger_pending_damage(mut commands: Commands, mut pending: ResMut<PendingDamage>) {
    for event in pending.events.drain(..) {
        commands.trigger(DbspDamageIngress::from(event));
    }
}

fn setup_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(DbspPlugin);
    let entity = app
        .world_mut()
        .spawn((
            DdlogId(1),
            Transform::default(),
            Health {
                current: 90,
                max: 100,
            },
        ))
        .id();
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
fn observer_events_ingest_damage_within_frame() {
    let (mut app, entity) = setup_app();
    app.insert_resource(PendingDamage {
        events: vec![sequenced_damage_tick_one()],
    });
    app.add_systems(Update, trigger_pending_damage);
    app.update();

    let health = app.world().get::<Health>(entity).expect("missing Health");
    assert_eq!(health.current, 60);
}

#[rstest]
fn duplicate_events_within_tick_apply_once() {
    let (mut app, entity) = setup_app();
    let damage = sequenced_damage_tick_one();
    app.insert_resource(PendingDamage {
        events: vec![damage, damage],
    });
    app.add_systems(Update, trigger_pending_damage);
    app.update();

    let health = app.world().get::<Health>(entity).expect("missing Health");
    assert_eq!(health.current, 60);
}
