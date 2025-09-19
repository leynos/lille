use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::{DamageInbox, DbspPlugin, DdlogId, Health};

#[test]
fn duplicate_damage_events_do_not_double_apply() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(DbspPlugin);

    let entity = app
        .world
        .spawn((
            DdlogId(1),
            Transform::default(),
            Health {
                current: 90,
                max: 100,
            },
        ))
        .id();

    // Prime the circuit with the entity state before sending damage.
    app.update();

    let damage = DamageEvent {
        entity: 1,
        amount: 30,
        source: DamageSource::External,
        at_tick: 1,
        seq: Some(42),
    };

    {
        let mut inbox = app.world.resource_mut::<DamageInbox>();
        inbox.push(damage);
        inbox.push(damage);
    }

    app.update();

    let health = app.world.get::<Health>(entity).unwrap();
    assert_eq!(health.current, 60);

    {
        let mut inbox = app.world.resource_mut::<DamageInbox>();
        inbox.push(damage);
        inbox.push(damage);
    }

    app.update();

    let health = app.world.get::<Health>(entity).unwrap();
    assert_eq!(health.current, 60);
}
