//! Behavioural tests for DBSP health integration.

use anyhow::{Context, Result};

use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::DbspState;
use lille::{DamageInbox, DbspPlugin, DdlogId, Health};

/// Encapsulates a Bevy app seeded with health fixtures.
struct HealthEnv {
    app: App,
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
        Self { app, entity }
    }

    fn push_damage_repeated(&mut self, event: DamageEvent, repeat: usize) -> Result<()> {
        let mut inbox = self
            .app
            .world
            .get_resource_mut::<DamageInbox>()
            .context("DamageInbox resource missing")?;
        for _ in 0..repeat {
            inbox.push(event);
        }
        Ok(())
    }

    fn push_damage(&mut self, event: DamageEvent) -> Result<()> {
        self.push_damage_repeated(event, 1)
    }

    fn push_damage_twice(&mut self, event: DamageEvent) -> Result<()> {
        self.push_damage_repeated(event, 2)
    }

    fn update(&mut self) -> Result<()> {
        self.app.update();
        Ok(())
    }

    fn current_health(&mut self) -> Result<u16> {
        let health = self
            .app
            .world
            .get::<Health>(self.entity)
            .context("entity missing Health component")?;
        Ok(health.current)
    }

    fn duplicate_count(&mut self) -> Result<u64> {
        let state = self
            .app
            .world
            .get_non_send_resource::<DbspState>()
            .context("DbspState non-send resource missing")?;
        Ok(state.applied_health_duplicates())
    }
}

impl Default for HealthEnv {
    fn default() -> Self {
        Self::new()
    }
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

#[test]
fn duplicate_events_within_tick_apply_once() -> Result<()> {
    let mut env = HealthEnv::new();
    let damage = sequenced_damage_tick_one();
    env.push_damage_twice(damage)?;
    env.update()?;
    assert_eq!(env.current_health()?, 60);
    assert_eq!(env.duplicate_count()?, 1);
    Ok(())
}

#[test]
fn replaying_same_tick_delta_is_ignored() -> Result<()> {
    let mut env = HealthEnv::new();
    let damage = sequenced_damage_tick_one();
    env.push_damage_twice(damage)?;
    env.update()?;
    env.push_damage(damage)?;
    env.update()?;
    assert_eq!(env.current_health()?, 60);
    assert_eq!(env.duplicate_count()?, 2);
    Ok(())
}

#[test]
fn new_ticks_consume_damage_once() -> Result<()> {
    let mut env = HealthEnv::new();
    let damage = sequenced_damage_tick_one();
    env.push_damage_twice(damage)?;
    env.update()?;
    env.push_damage(damage)?;
    env.update()?;
    let next_tick = sequenced_damage_tick_two();
    env.push_damage(next_tick)?;
    env.update()?;
    assert_eq!(env.current_health()?, 30);
    assert_eq!(env.duplicate_count()?, 2);
    Ok(())
}

#[test]
fn healing_saturates_at_max_health() -> Result<()> {
    let mut env = HealthEnv::new();
    let damage = sequenced_damage_tick_one();
    env.push_damage_twice(damage)?;
    env.update()?;
    env.push_damage(damage)?;
    env.update()?;
    let next_tick = sequenced_damage_tick_two();
    env.push_damage(next_tick)?;
    env.update()?;
    let heal = healing_event();
    env.push_damage(heal)?;
    env.update()?;
    assert_eq!(env.current_health()?, 100);
    assert_eq!(env.duplicate_count()?, 2);
    Ok(())
}

#[test]
fn healing_when_already_at_max_health_does_not_overflow() -> Result<()> {
    let mut env = HealthEnv::new();
    let initial_heal = healing_event();
    env.push_damage(initial_heal)?;
    env.update()?;
    assert_eq!(env.current_health()?, 100);

    let extra_heal = healing_at_max_event();
    env.push_damage(extra_heal)?;
    env.update()?;
    assert_eq!(env.current_health()?, 100);
    assert_eq!(env.duplicate_count()?, 0);
    Ok(())
}

#[test]
fn unsequenced_duplicates_are_filtered_within_tick() -> Result<()> {
    let mut env = HealthEnv::new();
    let damage = sequenced_damage_tick_one();
    env.push_damage_twice(damage)?;
    env.update()?;
    env.push_damage(damage)?;
    env.update()?;
    let next_tick = sequenced_damage_tick_two();
    env.push_damage(next_tick)?;
    env.update()?;
    let heal = healing_event();
    env.push_damage(heal)?;
    env.update()?;
    let unsequenced = unsequenced_damage();
    env.push_damage_twice(unsequenced)?;
    env.update()?;
    assert_eq!(env.current_health()?, 50);
    assert_eq!(env.duplicate_count()?, 3);
    Ok(())
}

#[test]
fn replaying_unsequenced_deltas_for_same_tick_is_ignored() -> Result<()> {
    let mut env = HealthEnv::new();
    let damage = sequenced_damage_tick_one();
    env.push_damage_twice(damage)?;
    env.update()?;
    env.push_damage(damage)?;
    env.update()?;
    let next_tick = sequenced_damage_tick_two();
    env.push_damage(next_tick)?;
    env.update()?;
    let heal = healing_event();
    env.push_damage(heal)?;
    env.update()?;
    let unsequenced = unsequenced_damage();
    env.push_damage_twice(unsequenced)?;
    env.update()?;
    env.push_damage(unsequenced)?;
    env.update()?;
    assert_eq!(env.current_health()?, 50);
    assert_eq!(env.duplicate_count()?, 4);
    Ok(())
}
