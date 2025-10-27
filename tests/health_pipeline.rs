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

#[derive(Clone, Copy)]
struct ExpectedHealthState {
    health: u16,
    health_message: &'static str,
    duplicates: u64,
    duplicates_message: &'static str,
}

/// Assert both health state and duplicate counter with custom messages.
fn assert_health_state(env: &HealthEnv, expected: ExpectedHealthState) -> Result<()> {
    ensure!(
        env.current_health()? == expected.health,
        "{}",
        expected.health_message
    );
    ensure!(
        env.duplicate_count()? == expected.duplicates,
        "{}",
        expected.duplicates_message
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

/// Push unsequenced damage twice within the same tick and update.
fn apply_unsequenced_damage_within_tick(env: &mut HealthEnv, event: DamageEvent) -> Result<()> {
    env.push_damage_twice(event)?;
    env.update()
}

/// Push unsequenced damage twice, update, then replay it once more.
fn apply_and_replay_unsequenced_damage(env: &mut HealthEnv, event: DamageEvent) -> Result<()> {
    env.push_damage_twice(event)?;
    env.update()?;
    env.push_damage(event)?;
    env.update()
}

type PostPlanFn = fn(&mut HealthEnv) -> Result<()>;

#[expect(
    clippy::unnecessary_wraps,
    reason = "Signature aligns with other post-plan handlers for uniform use with ?"
)]
#[expect(
    clippy::missing_const_for_fn,
    reason = "Handler operates on runtime state and cannot be const"
)]
fn no_follow_up(_: &mut HealthEnv) -> Result<()> {
    Ok(())
}

fn apply_unsequenced_duplicates(env: &mut HealthEnv) -> Result<()> {
    let unsequenced = unsequenced_damage();
    apply_unsequenced_damage_within_tick(env, unsequenced)
}

fn apply_unsequenced_duplicates_with_replay(env: &mut HealthEnv) -> Result<()> {
    let unsequenced = unsequenced_damage();
    apply_and_replay_unsequenced_damage(env, unsequenced)
}

#[rstest]
fn duplicate_events_within_tick_apply_once(mut health_env: HealthEnv) -> Result<()> {
    let damage = sequenced_damage_tick_one();
    health_env.push_damage_twice(damage)?;
    health_env.update()?;
    assert_health_state(
        &health_env,
        ExpectedHealthState {
            health: 60,
            health_message: "health should drop to 60 after consuming duplicate damage",
            duplicates: 1,
            duplicates_message: "duplicate counter should record a single entry",
        },
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
        ExpectedHealthState {
            health: 60,
            health_message: "health should stay at 60 when replaying the same tick",
            duplicates: 2,
            duplicates_message: "duplicate counter should record both replays",
        },
    )
}

#[rstest]
#[case::new_ticks_consume_damage_once(
    DamageSequencePlan {
        replay_same_tick: true,
        next_tick: Some(sequenced_damage_tick_two()),
        ..DamageSequencePlan::default()
    },
    no_follow_up,
    ExpectedHealthState {
        health: 30,
        health_message: "health should drop to 30 after consuming new tick damage once",
        duplicates: 2,
        duplicates_message: "duplicate counter should ignore unique tick",
    }
)]
#[case::healing_saturates_at_max(
    DamageSequencePlan {
        replay_same_tick: true,
        next_tick: Some(sequenced_damage_tick_two()),
        heal: Some(healing_event()),
    },
    no_follow_up,
    ExpectedHealthState {
        health: 100,
        health_message: "healing should saturate at maximum health",
        duplicates: 2,
        duplicates_message: "duplicate counter should remain unchanged after healing",
    }
)]
#[case::unsequenced_duplicates_filtered(
    DamageSequencePlan {
        replay_same_tick: true,
        next_tick: Some(sequenced_damage_tick_two()),
        heal: Some(healing_event()),
    },
    apply_unsequenced_duplicates,
    ExpectedHealthState {
        health: 50,
        health_message: "unsequenced duplicates within a tick should only apply once",
        duplicates: 3,
        duplicates_message: "duplicate counter should include unsequenced duplicates",
    }
)]
#[case::unsequenced_replays_ignored(
    DamageSequencePlan {
        replay_same_tick: true,
        next_tick: Some(sequenced_damage_tick_two()),
        heal: Some(healing_event()),
    },
    apply_unsequenced_duplicates_with_replay,
    ExpectedHealthState {
        health: 50,
        health_message: "replayed unsequenced deltas should not further change health",
        duplicates: 4,
        duplicates_message: "duplicate counter should reflect the additional replayed delta",
    }
)]
fn damage_sequence_behaviour(
    mut health_env: HealthEnv,
    #[case] plan: DamageSequencePlan,
    #[case] post_plan: PostPlanFn,
    #[case] expected: ExpectedHealthState,
) -> Result<()> {
    apply_damage_sequence(&mut health_env, sequenced_damage_tick_one(), plan)?;
    post_plan(&mut health_env)?;
    assert_health_state(&health_env, expected)
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
        ExpectedHealthState {
            health: 100,
            health_message: "additional healing should not overflow maximum health",
            duplicates: 0,
            duplicates_message: "duplicate counter should remain zero without replayed events",
        },
    )
}
