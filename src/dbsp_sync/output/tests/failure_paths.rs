//! Step-failure path tests for the DBSP output application system.
//!
//! Covers the error event, clearing buffered inputs so they cannot replay, and
//! rolling back the health/damage tracking the cache system advanced when a
//! step fails mid-frame.

use super::*;
use crate::dbsp_sync::DamageInbox;
use rstest::fixture;

/// An app wired with the DBSP plugin and an error-capturing observer, flushed
/// and ready for `app.update()`. Shared by the full-pipeline failure tests so
/// they exercise `cache_state_for_dbsp_system` and `apply_dbsp_outputs_system`
/// in their normal chained order.
#[fixture]
fn plugin_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    dbsp_test_support::install_error_observer(&mut app);
    app.add_plugins(DbspPlugin);
    app.world_mut().flush();
    app
}

/// Makes the next `step_circuit()` call fail, without touching DBSP logic.
fn force_step_failure(app: &mut App) {
    app.world_mut()
        .non_send_resource_mut::<DbspState>()
        .set_stepper_for_testing(force_step_error);
}

/// Restores the real stepper after a forced failure.
fn restore_stepper(app: &mut App) {
    app.world_mut()
        .non_send_resource_mut::<DbspState>()
        .set_stepper_for_testing(try_step);
}

#[rstest]
fn step_failure_triggers_error_event(#[from(plugin_app)] mut app: App) {
    // Run startup to initialise WorldHandle before priming state.
    app.update();

    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);

    force_step_failure(&mut app);

    app.update();

    let step_errors = app.world().resource::<dbsp_test_support::CapturedErrors>();
    let error = step_errors
        .0
        .first()
        .expect("DBSP error event should be captured");
    assert_eq!(error.0, format!("{:?}", DbspSyncErrorContext::Step));
    assert!(error.1.contains("forced failure"));

    let transform = app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform should remain after failed step");
    assert_eq!(transform.translation, Vec3::ZERO);
}

#[rstest]
fn failed_step_clears_inputs_so_they_do_not_replay() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);

    // First run fails to step; the system must still clear circuit inputs so
    // the buffered records cannot replay on a later, successful tick.
    force_step_failure(&mut app);
    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("system should run even when the step fails");

    // Restore a working stepper and run again. Because the failed run cleared
    // the inputs, nothing is stepped and the transform stays at the origin.
    restore_stepper(&mut app);
    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let transform = app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform should remain after retry");
    assert_eq!(
        transform.translation,
        Vec3::ZERO,
        "stale position inputs must not replay after a failed step"
    );

    // The primed velocity_in must have been cleared alongside position_in: the
    // velocity stays at its spawned default rather than replaying the input.
    let velocity = app
        .world()
        .entity(entity)
        .get::<VelocityComp>()
        .expect("VelocityComp should remain after retry");
    let default = VelocityComp::default();
    assert_eq!(
        (velocity.vx, velocity.vy, velocity.vz),
        (default.vx, default.vy, default.vz),
        "stale velocity inputs must not replay after a failed step"
    );
}

#[rstest]
fn failed_step_rolls_back_health_tracking(#[from(plugin_app)] mut app: App) {
    // Full-pipeline test: the cache system advances `health_snapshot` before the
    // output system steps the circuit. If the step fails, clearing the circuit
    // inputs must be paired with rolling that tracking back, or the next frame
    // emits phantom retractions for records the circuit never accepted.

    // Startup, then a first successful frame to establish the live health
    // snapshot both in DBSP and in the Rust-side bookkeeping.
    app.update();
    let entity = spawn_entity(&mut app);
    app.update();

    let snapshot_before = app
        .world()
        .non_send_resource::<DbspState>()
        .health_snapshot
        .clone();
    assert!(
        snapshot_before.contains_key(&1),
        "the first frame should record a health snapshot for entity 1"
    );

    // Mutate health so the next cache pass computes a new snapshot, then force
    // the step to fail on that frame.
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        let mut health = entity_mut
            .get_mut::<Health>()
            .expect("spawned entity should have a Health component");
        health.current = 50;
    }
    force_step_failure(&mut app);
    app.update();

    let snapshot_after = app
        .world()
        .non_send_resource::<DbspState>()
        .health_snapshot
        .clone();
    assert_eq!(
        snapshot_after, snapshot_before,
        "a failed step must roll back health_snapshot to its pre-frame value so \
         the next frame does not emit phantom health-state retractions"
    );
}

#[rstest]
fn failed_step_rolls_back_unsequenced_damage_dedupe_state(#[from(plugin_app)] mut app: App) {
    // Full-pipeline sibling of `failed_step_rolls_back_health_tracking`, driving
    // the real cache-to-step failure path for `applied_unsequenced` (the
    // unsequenced-damage dedupe state) rather than calling
    // `record_unsequenced_undo` directly. The cache system ingests an
    // unsequenced `DamageEvent` from the `DamageInbox` and advances
    // `applied_unsequenced`; a failed step must restore it to its exact
    // pre-frame value. This covers the absent-before-frame case: the entry the
    // cache pass adds must be gone again after the rollback.

    app.update(); // startup
    spawn_entity(&mut app); // DdlogId(1)
    app.update(); // register the entity in id_map (no damage yet)

    // Pre-frame: entity 1 has no `applied_unsequenced` entry.
    let before = app
        .world()
        .non_send_resource::<DbspState>()
        .applied_unsequenced
        .clone();
    assert!(
        !before.contains_key(&1),
        "entity 1 must be absent from applied_unsequenced before the failing frame"
    );

    // Queue one unsequenced damage event and force the step to fail. The cache
    // pass will call `record_unsequenced_undo(1)` (capturing the absent entry)
    // and then add entity 1 via `record_duplicate_unsequenced_damage`.
    app.world_mut()
        .resource_mut::<DamageInbox>()
        .push(unsequenced_damage(10));
    force_step_failure(&mut app);
    app.update();

    // The failing frame's cache pass really did run and consume the queued
    // event — without this, an inbox that was never drained would leave
    // `applied_unsequenced` untouched and the rollback assertion would hold
    // vacuously.
    assert!(
        app.world().resource::<DamageInbox>().is_empty(),
        "the failing frame's cache pass must have drained the DamageInbox"
    );

    let after = app
        .world()
        .non_send_resource::<DbspState>()
        .applied_unsequenced
        .clone();
    // Restored to the exact pre-frame state. If `ingest_damage_events` stopped
    // calling `record_unsequenced_undo` before the dedupe mutation, the undo log
    // would omit entity 1, the rollback could not remove the entry it added, and
    // both assertions below would fail.
    assert_eq!(
        after, before,
        "a failed step must roll back applied_unsequenced to its exact pre-frame state"
    );
    assert!(
        !after.contains_key(&1),
        "the entry the rolled-back frame added must be absent afterwards"
    );
}

#[rstest]
fn rolled_back_retraction_markers_do_not_suppress_a_later_delta() {
    // A frame's cache pass records `expected_health_retractions` markers for the
    // damage retractions it issues, and `should_apply_health_delta` consumes a
    // matching marker to suppress the echoing delta. When the step fails those
    // retractions are discarded with the circuit inputs, so the markers must go
    // too — otherwise a later, legitimate delta for the same
    // `(entity, at_tick, seq)` would be silently swallowed.
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    // Emits a delta for entity 1 at tick 1, seq 1, taking health 90 -> 40 (the
    // same inputs `applies_outputs_updates_components` asserts are applied).
    push_health_inputs(&mut app, 90, 50);

    // Stand in for a failed frame that left a marker matching that delta, then
    // roll the frame back.
    {
        let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.expected_health_retractions.insert((1, 1, Some(1)));
        state.rollback_frame_tracking();
    }

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let health = app
        .world()
        .entity(entity)
        .get::<Health>()
        .expect("Health component should remain present");
    assert_eq!(
        health.current, 40,
        "a rolled-back frame's retraction markers must not suppress a later \
         matching health delta"
    );
}

#[rstest]
fn step_failures_and_skipped_outputs_are_counted() {
    // The bounded reliability counters must actually move: a forced step
    // failure increments `step_failures`, and a retracted (negative-weight)
    // output record increments `skipped_outputs`. Each phase gets its own
    // scope so the two setups do not shadow one another.
    {
        let mut app = setup_app().expect("failed to set up test app");
        let entity = spawn_entity(&mut app);
        prime_entity_mapping(&mut app, entity);
        assert_eq!(
            app.world().non_send_resource::<DbspState>().step_failures(),
            0,
            "a fresh state reports no step failures"
        );

        // A failed step is counted once.
        force_step_failure(&mut app);
        app.world_mut()
            .run_system_once(apply_dbsp_outputs_system)
            .expect("system should run even when the step fails");
        assert_eq!(
            app.world().non_send_resource::<DbspState>().step_failures(),
            1,
            "a failed step must be counted"
        );
    }

    // A retracted position produces a non-positive-weight output record, which
    // the weight gate skips and counts. This needs a fresh app: the failed step
    // above cleared the circuit inputs, including the primed block, so the same
    // app would emit no output at all to skip. These are
    // `negative_weight_position_is_not_applied`'s inputs — a velocity at weight
    // +1 so output is produced, and the position retracted at weight -1.
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_entity_mapping(&mut app, entity);
    push_velocity_input(
        &mut app,
        Velocity {
            entity: 1,
            vx: 1.0.into(),
            vy: 0.0.into(),
            vz: 0.0.into(),
        },
        1,
    );
    push_position_input(
        &mut app,
        Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: 1.0.into(),
        },
        -1,
    );
    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");
    assert!(
        app.world()
            .non_send_resource::<DbspState>()
            .skipped_outputs()
            > 0,
        "a negative-weight output record must be counted as skipped"
    );
}

/// An unsequenced (`seq: None`) external damage event for entity 1 at tick 1.
fn unsequenced_damage(amount: u16) -> DamageEvent {
    DamageEvent {
        entity: 1,
        amount,
        source: DamageSource::External,
        at_tick: 1,
        seq: None,
    }
}
