//! Behavioural tests for DBSP health integration.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::DbspState;
use lille::{DamageInbox, DbspPlugin, DdlogId, Health};

#[derive(Clone, Debug)]
struct HealthEnv {
    app: Arc<Mutex<App>>,
    entity: Entity,
}

impl HealthEnv {
    fn new() -> Self {
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
        // Prime the circuit so the initial health snapshot is registered.
        app.update();
        Self {
            app: Arc::new(Mutex::new(app)),
            entity,
        }
    }

    fn push_damage_repeated(&self, event: DamageEvent, repeat: usize) {
        let mut app = self.app.lock().expect("lock app");
        let mut inbox = app.world.resource_mut::<DamageInbox>();
        for _ in 0..repeat {
            inbox.push(event);
        }
    }

    fn push_damage(&self, event: DamageEvent) {
        self.push_damage_repeated(event, 1);
    }

    fn push_damage_twice(&self, event: DamageEvent) {
        self.push_damage_repeated(event, 2);
    }

    fn update(&self) {
        self.app.lock().expect("lock app").update();
    }

    fn current_health(&self) -> u16 {
        let app = self.app.lock().expect("lock app");
        app.world
            .get::<Health>(self.entity)
            .expect("entity has health")
            .current
    }

    fn duplicate_count(&self) -> u64 {
        let app = self.app.lock().expect("lock app");
        app.world
            .non_send_resource::<DbspState>()
            .applied_health_duplicates()
    }
}

impl Default for HealthEnv {
    fn default() -> Self {
        Self::new()
    }
}

fn run_rspec_serial<T>(suite: &rspec::block::suite::Suite<T>)
where
    T: Clone + Send + Sync + std::fmt::Debug,
{
    let logger = Arc::new(rspec::Logger::new(std::io::stdout()));
    let configuration = rspec::ConfigurationBuilder::default()
        .parallel(false)
        .build()
        .expect("build sequential rspec configuration");
    let runner = rspec::Runner::new(configuration, vec![logger]);
    runner.run(suite);
}

#[test]
fn duplicate_damage_events_do_not_double_apply() {
    run_rspec_serial(&rspec::given(
        "duplicate health deltas are idempotent",
        HealthEnv::default(),
        |ctx| {
            ctx.then("duplicate events within a tick apply once", |env| {
                let damage = DamageEvent {
                    entity: 1,
                    amount: 30,
                    source: DamageSource::External,
                    at_tick: 1,
                    seq: Some(42),
                };
                env.push_damage_twice(damage);
                env.update();
                assert_eq!(env.current_health(), 60);
                assert_eq!(env.duplicate_count(), 1);
            });
            ctx.then("replaying the same tick delta is ignored", |env| {
                let damage = DamageEvent {
                    entity: 1,
                    amount: 30,
                    source: DamageSource::External,
                    at_tick: 1,
                    seq: Some(42),
                };
                env.push_damage(damage);
                env.update();
                assert_eq!(env.current_health(), 60);
                assert_eq!(env.duplicate_count(), 2);
            });
            ctx.then("new ticks consume damage once", |env| {
                let damage = DamageEvent {
                    entity: 1,
                    amount: 30,
                    source: DamageSource::External,
                    at_tick: 2,
                    seq: Some(43),
                };
                env.push_damage(damage);
                env.update();
                assert_eq!(env.current_health(), 30);
                assert_eq!(env.duplicate_count(), 2);
            });
            ctx.then("healing saturates at max health", |env| {
                let heal = DamageEvent {
                    entity: 1,
                    amount: 80,
                    source: DamageSource::Script,
                    at_tick: 3,
                    seq: Some(5),
                };
                env.push_damage(heal);
                env.update();
                assert_eq!(env.current_health(), 100);
                assert_eq!(env.duplicate_count(), 2);
            });
            ctx.then("unsequenced duplicates are filtered within a tick", |env| {
                let damage = DamageEvent {
                    entity: 1,
                    amount: 50,
                    source: DamageSource::External,
                    at_tick: 4,
                    seq: None,
                };
                env.push_damage_twice(damage);
                env.update();
                assert_eq!(env.current_health(), 50);
                assert_eq!(env.duplicate_count(), 3);
            });
            ctx.then(
                "replaying unsequenced deltas for the same tick is ignored",
                |env| {
                    let damage = DamageEvent {
                        entity: 1,
                        amount: 50,
                        source: DamageSource::External,
                        at_tick: 4,
                        seq: None,
                    };
                    env.push_damage(damage);
                    env.update();
                    assert_eq!(env.current_health(), 50);
                    assert_eq!(env.duplicate_count(), 4);
                },
            );
        },
    ));
}
