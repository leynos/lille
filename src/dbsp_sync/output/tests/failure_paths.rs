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
