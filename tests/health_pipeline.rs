//! Behavioural tests for DBSP health integration.

use anyhow::{ensure, Context, Result};

use bevy::prelude::*;
use lille::dbsp_circuit::{DamageEvent, DamageSource};
use lille::dbsp_sync::DbspState;
use lille::{DamageInbox, DbspPlugin, DdlogId, Health};
use rstest::{fixture, rstest};

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
        inbox.extend(std::iter::repeat_n(event, repeat));
        Ok(())
    }

    fn push_damage(&mut self, event: DamageEvent) -> Result<()> {
        self.push_damage_repeated(event, 1)
    }

    fn push_damage_twice(&mut self, event: DamageEvent) -> Result<()> {
        self.push_damage_repeated(event, 2)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "Test helpers uniformly return Result to propagate setup failures."
    )]
    fn update(&mut self) -> Result<()> {
        self.app.update();
        Ok(())
    }

    fn current_health(&self) -> Result<u16> {
        let health = self
            .app
            .world
            .get::<Health>(self.entity)
            .context("entity missing Health component")?;
        Ok(health.current)
    }

    fn duplicate_count(&self) -> Result<u64> {
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

#[fixture]
fn health_env() -> HealthEnv {
    HealthEnv::new()
}

/// Assert both health state and duplicate counter with custom messages.
#[expect(
    clippy::too_many_arguments,
    reason = "Test helper intentionally packages health and duplicate assertions."
)]
fn assert_health_state(
    env: &HealthEnv,
    expected_health: u16,
    health_message: &str,
    expected_duplicates: u64,
    duplicates_message: &str,
) -> Result<()> {
    ensure!(env.current_health()? == expected_health, "{health_message}");
    ensure!(
        env.duplicate_count()? == expected_duplicates,
        "{duplicates_message}"
    );
    Ok(())
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

const fn sequenced_damage_tick_two() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 30,
        source: DamageSource::External,
        at_tick: 2,
        seq: Some(43),
    }
}

const fn healing_event() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 80,
        source: DamageSource::Script,
        at_tick: 3,
        seq: Some(5),
    }
}

const fn healing_at_max_event() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 50,
        source: DamageSource::Script,
        at_tick: 4,
        seq: Some(6),
    }
}

const fn unsequenced_damage() -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount: 50,
        source: DamageSource::External,
        at_tick: 4,
        seq: None,
    }
}

#[derive(Default, Clone, Copy)]
struct DamageSequencePlan {
    replay_same_tick: bool,
    next_tick: Option<DamageEvent>,
    heal: Option<DamageEvent>,
}

fn apply_damage_sequence(
    env: &mut HealthEnv,
    base: DamageEvent,
    plan: DamageSequencePlan,
) -> Result<()> {
    env.push_damage_twice(base)?;
    env.update()?;

    if plan.replay_same_tick {
        env.push_damage(base)?;
        env.update()?;
    }

    if let Some(next) = plan.next_tick {
        env.push_damage(next)?;
        env.update()?;
    }

    if let Some(heal_event) = plan.heal {
        env.push_damage(heal_event)?;
        env.update()?;
    }

    Ok(())
}

#[rstest]
fn duplicate_events_within_tick_apply_once(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    health_env.push_damage_twice(damage)?;
    health_env.update()?;
    assert_health_state(
        &health_env,
        60,
        "health should drop to 60 after consuming duplicate damage",
        1,
        "duplicate counter should record a single entry",
    )?;
    Ok(())
}

#[rstest]
fn replaying_same_tick_delta_is_ignored(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    apply_damage_sequence(
        &mut health_env,
        damage,
        DamageSequencePlan {
            replay_same_tick: true,
            ..DamageSequencePlan::default()
        },
    )?;
    assert_health_state(
        &health_env,
        60,
        "health should stay at 60 when replaying the same tick",
        2,
        "duplicate counter should record both replays",
    )
}

#[rstest]
fn new_ticks_consume_damage_once(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    let next_tick = sequenced_damage_tick_two();
    apply_damage_sequence(
        &mut health_env,
        damage,
        DamageSequencePlan {
            replay_same_tick: true,
            next_tick: Some(next_tick),
            ..DamageSequencePlan::default()
        },
    )?;
    assert_health_state(
        &health_env,
        30,
        "health should drop to 30 after consuming new tick damage once",
        2,
        "duplicate counter should ignore unique tick",
    )
}

#[rstest]
fn healing_saturates_at_max_health(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    let next_tick = sequenced_damage_tick_two();
    let heal = healing_event();
    apply_damage_sequence(
        &mut health_env,
        damage,
        DamageSequencePlan {
            replay_same_tick: true,
            next_tick: Some(next_tick),
            heal: Some(heal),
        },
    )?;
    assert_health_state(
        &health_env,
        100,
        "healing should saturate at maximum health",
        2,
        "duplicate counter should remain unchanged after healing",
    )
}

#[rstest]
fn healing_when_already_at_max_health_does_not_overflow(mut health_env: HealthEnv) -> Result<()> {
    let initial_heal = healing_event();
    health_env.push_damage(initial_heal)?;
    health_env.update()?;

    let extra_heal = healing_at_max_event();
    health_env.push_damage(extra_heal)?;
    health_env.update()?;
    assert_health_state(
        &health_env,
        100,
        "additional healing should not overflow maximum health",
        0,
        "duplicate counter should remain zero without replayed events",
    )
}

#[rstest]
fn unsequenced_duplicates_are_filtered_within_tick(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    let next_tick = sequenced_damage_tick_two();
    let heal = healing_event();
    apply_damage_sequence(
        &mut health_env,
        damage,
        DamageSequencePlan {
            replay_same_tick: true,
            next_tick: Some(next_tick),
            heal: Some(heal),
        },
    )?;
    let unsequenced = unsequenced_damage();
    health_env.push_damage_twice(unsequenced)?;
    health_env.update()?;
    assert_health_state(
        &health_env,
        50,
        "unsequenced duplicates within a tick should only apply once",
        3,
        "duplicate counter should include unsequenced duplicates",
    )
}

#[rstest]
fn replaying_unsequenced_deltas_for_same_tick_is_ignored(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    let next_tick = sequenced_damage_tick_two();
    let heal = healing_event();
    apply_damage_sequence(
        &mut health_env,
        damage,
        DamageSequencePlan {
            replay_same_tick: true,
            next_tick: Some(next_tick),
            heal: Some(heal),
        },
    )?;
    let unsequenced = unsequenced_damage();
    health_env.push_damage_twice(unsequenced)?;
    health_env.update()?;
    health_env.push_damage(unsequenced)?;
    health_env.update()?;
    assert_health_state(
        &health_env,
        50,
        "replayed unsequenced deltas should not further change health",
        4,
        "duplicate counter should reflect the additional replayed delta",
    )
}
