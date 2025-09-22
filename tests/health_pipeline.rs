//! Behavioural tests for DBSP health integration.

use std::cell::RefCell;
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

fn sequenced_damage_tick_one() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 30,
        source: DamageSource::External,
        at_tick: 1,
        seq: Some(42),
    }
}

fn sequenced_damage_tick_two() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 30,
        source: DamageSource::External,
        at_tick: 2,
        seq: Some(43),
    }
}

fn healing_event() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 80,
        source: DamageSource::Script,
        at_tick: 3,
        seq: Some(5),
    }
}

fn healing_at_max_event() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 50,
        source: DamageSource::Script,
        at_tick: 4,
        seq: Some(6),
    }
}

fn unsequenced_damage() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 50,
        source: DamageSource::External,
        at_tick: 4,
        seq: None,
    }
}

fn run_health_test<F>(test_name: &'static str, test_fn: F)
where
    F: FnOnce(&mut HealthEnv) + 'static,
{
    run_rspec_serial(&rspec::given(
        "duplicate health deltas are idempotent",
        HealthEnv::default(),
        |ctx| {
            let test_fn_cell = RefCell::new(Some(test_fn));
            ctx.then(test_name, move |env| {
                let mut env = env.clone();
                let scenario = {
                    let mut borrowed = test_fn_cell.borrow_mut();
                    borrowed
                        .take()
                        .expect("health scenario should execute once")
                };
                scenario(&mut env);
            });
        },
    ));
}

#[test]
fn duplicate_events_within_tick_apply_once() {
    run_health_test("duplicate events within a tick apply once", |env| {
        let damage = sequenced_damage_tick_one();
        env.push_damage_twice(damage);
        env.update();
        assert_eq!(env.current_health(), 60);
        assert_eq!(env.duplicate_count(), 1);
    });
}

#[test]
fn replaying_same_tick_delta_is_ignored() {
    run_health_test("replaying the same tick delta is ignored", |env| {
        let damage = sequenced_damage_tick_one();
        env.push_damage_twice(damage);
        env.update();
        env.push_damage(damage);
        env.update();
        assert_eq!(env.current_health(), 60);
        assert_eq!(env.duplicate_count(), 2);
    });
}

#[test]
fn new_ticks_consume_damage_once() {
    run_health_test("new ticks consume damage once", |env| {
        let damage = sequenced_damage_tick_one();
        env.push_damage_twice(damage);
        env.update();
        env.push_damage(damage);
        env.update();
        let next_tick = sequenced_damage_tick_two();
        env.push_damage(next_tick);
        env.update();
        assert_eq!(env.current_health(), 30);
        assert_eq!(env.duplicate_count(), 2);
    });
}

#[test]
fn healing_saturates_at_max_health() {
    run_health_test("healing saturates at max health", |env| {
        let damage = sequenced_damage_tick_one();
        env.push_damage_twice(damage);
        env.update();
        env.push_damage(damage);
        env.update();
        let next_tick = sequenced_damage_tick_two();
        env.push_damage(next_tick);
        env.update();
        let heal = healing_event();
        env.push_damage(heal);
        env.update();
        assert_eq!(env.current_health(), 100);
        assert_eq!(env.duplicate_count(), 2);
    });
}

#[test]
fn healing_when_already_at_max_health_does_not_overflow() {
    run_health_test(
        "healing when already at max health does not overflow",
        |env| {
            let initial_heal = healing_event();
            env.push_damage(initial_heal);
            env.update();
            assert_eq!(env.current_health(), 100);

            let extra_heal = healing_at_max_event();
            env.push_damage(extra_heal);
            env.update();
            assert_eq!(env.current_health(), 100);
            assert_eq!(env.duplicate_count(), 0);
        },
    );
}

#[test]
fn unsequenced_duplicates_are_filtered_within_tick() {
    run_health_test("unsequenced duplicates are filtered within a tick", |env| {
        let damage = sequenced_damage_tick_one();
        env.push_damage_twice(damage);
        env.update();
        env.push_damage(damage);
        env.update();
        let next_tick = sequenced_damage_tick_two();
        env.push_damage(next_tick);
        env.update();
        let heal = healing_event();
        env.push_damage(heal);
        env.update();
        let unsequenced = unsequenced_damage();
        env.push_damage_twice(unsequenced);
        env.update();
        assert_eq!(env.current_health(), 50);
        assert_eq!(env.duplicate_count(), 3);
    });
}

#[test]
fn replaying_unsequenced_deltas_for_same_tick_is_ignored() {
    run_health_test(
        "replaying unsequenced deltas for the same tick is ignored",
        |env| {
            let damage = sequenced_damage_tick_one();
            env.push_damage_twice(damage);
            env.update();
            env.push_damage(damage);
            env.update();
            let next_tick = sequenced_damage_tick_two();
            env.push_damage(next_tick);
            env.update();
            let heal = healing_event();
            env.push_damage(heal);
            env.update();
            let unsequenced = unsequenced_damage();
            env.push_damage_twice(unsequenced);
            env.update();
            env.push_damage(unsequenced);
            env.update();
            assert_eq!(env.current_health(), 50);
            assert_eq!(env.duplicate_count(), 4);
        },
    );
}
