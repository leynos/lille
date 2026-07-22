//! Step-failure path tests for the DBSP output application system.
//!
//! Covers the error event, clearing buffered inputs so they cannot replay, and
//! rolling back the health/damage tracking the cache system advanced when a
//! step fails mid-frame.

use super::*;

#[rstest]
fn step_failure_triggers_error_event() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    dbsp_test_support::install_error_observer(&mut app);
    app.add_plugins(DbspPlugin);
    app.world_mut().flush();

    // Run startup to initialise WorldHandle before priming state.
    app.update();

    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);

    {
        let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.set_stepper_for_testing(force_step_error);
    }

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
    {
        let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.set_stepper_for_testing(force_step_error);
    }
    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("system should run even when the step fails");

    // Restore a working stepper and run again. Because the failed run cleared
    // the inputs, nothing is stepped and the transform stays at the origin.
    {
        let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.set_stepper_for_testing(try_step);
    }
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
fn failed_step_rolls_back_health_tracking() {
    // Full-pipeline test: the cache system advances `health_snapshot` before the
    // output system steps the circuit. If the step fails, clearing the circuit
    // inputs must be paired with rolling that tracking back, or the next frame
    // emits phantom retractions for records the circuit never accepted.
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    dbsp_test_support::install_error_observer(&mut app);
    app.add_plugins(DbspPlugin);
    app.world_mut().flush();

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
    {
        let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.set_stepper_for_testing(force_step_error);
    }
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
