//! Edge-case tests for the DBSP output application systems.
//!
//! Covers conversion bounds and the guard paths that skip records for
//! unmappable, unknown, despawned, or component-less entities.

use super::*;

#[rstest]
#[case::nan(f64::NAN, None)]
#[case::positive_infinity(f64::INFINITY, None)]
#[case::negative_infinity(f64::NEG_INFINITY, None)]
#[case::above_f32_range(1e300, None)]
#[case::below_f32_range(-1e300, None)]
#[case::in_range(1.5, Some(1.5))]
fn f32_from_f64_bounds(#[case] value: f64, #[case] expected: Option<f32>) {
    assert_eq!(f32_from_f64(value), expected);
}

#[rstest]
fn out_of_range_outputs_leave_components_unchanged() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    // Prime the identifier maps and floor geometry by hand: unlike
    // `prime_state`, no in-range position or velocity records are pushed, so
    // the only circuit outputs are the out-of-range ones under test.
    {
        let mut state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.id_map.insert(1, entity);
        state.rev_map.insert(entity, 1);
        state.circuit.block_in().push(
            Block {
                id: 1,
                x: 0,
                y: 0,
                z: 0,
            },
            1,
        );
        state.circuit.position_in().push(
            Position {
                entity: 1,
                x: 1e300.into(),
                y: (-1e300).into(),
                z: 1.0.into(),
            },
            1,
        );
        state.circuit.velocity_in().push(
            Velocity {
                entity: 1,
                vx: 1e300.into(),
                vy: (-1e300).into(),
                vz: 0.0.into(),
            },
            1,
        );
    }

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let transform = app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform component should remain after applying DBSP outputs");
    assert_eq!(transform.translation.x, 0.0);
    assert_eq!(transform.translation.y, 0.0);
    let velocity = app
        .world()
        .entity(entity)
        .get::<VelocityComp>()
        .expect("Velocity component should remain after applying DBSP outputs");
    assert_eq!(velocity.vx, 0.0);
    assert_eq!(velocity.vy, 0.0);
}

#[rstest]
fn health_delta_for_unmappable_entity_is_skipped() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    push_health_inputs_for(&mut app, u64::MAX, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let health = app
        .world()
        .entity(entity)
        .get::<Health>()
        .expect("Health component should remain after applying DBSP outputs");
    assert_eq!(health.current, 90);
}

#[rstest]
fn health_delta_for_unknown_entity_is_skipped() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    push_health_inputs_for(&mut app, 2, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let health = app
        .world()
        .entity(entity)
        .get::<Health>()
        .expect("Health component should remain after applying DBSP outputs");
    assert_eq!(health.current, 90);
}

#[rstest]
fn health_delta_without_health_component_is_skipped() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = app
        .world_mut()
        .spawn((DdlogId(1), Transform::default(), VelocityComp::default()))
        .id();
    prime_state(&mut app, entity);
    push_health_inputs(&mut app, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    assert!(
        app.world().entity(entity).get::<Health>().is_none(),
        "entity should still lack a Health component"
    );
    let state = app.world().non_send_resource::<DbspState>();
    assert_eq!(state.applied_health_duplicates(), 0);
}

#[rstest]
fn health_delta_clamps_to_zero_on_overkill() {
    let mut app = setup_app().expect("failed to set up test app");
    // Give the ECS entity less health than the circuit's snapshot so the
    // applied delta drives the raw value negative and exercises clamping.
    let entity = app
        .world_mut()
        .spawn((
            DdlogId(1),
            Transform::default(),
            VelocityComp::default(),
            Health {
                current: 10,
                max: 100,
            },
        ))
        .id();
    prime_state(&mut app, entity);
    push_health_inputs(&mut app, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let health = app
        .world()
        .entity(entity)
        .get::<Health>()
        .expect("Health component should remain after applying DBSP outputs");
    assert_eq!(health.current, 0);
}

#[rstest]
fn health_delta_for_despawned_entity_is_skipped() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    push_health_inputs(&mut app, 90, 50);
    app.world_mut().despawn(entity);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let state = app.world().non_send_resource::<DbspState>();
    assert_eq!(state.applied_health_duplicates(), 0);
}
