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
    assert!(transform.translation.x.abs() < f32::EPSILON);
    assert!(transform.translation.y.abs() < f32::EPSILON);
    let velocity = app
        .world()
        .entity(entity)
        .get::<VelocityComp>()
        .expect("Velocity component should remain after applying DBSP outputs");
    assert!(velocity.vx.abs() < f32::EPSILON);
    assert!(velocity.vy.abs() < f32::EPSILON);
}

// Both cases exercise distinct early-return branches of `resolve_health_target`:
// `u64::MAX` fails `i64::try_from` (unmappable), while `2` converts but misses
// `DbspState::id_map` (unknown). Neither may mutate the mapped entity's Health.
#[rstest]
#[case::unmappable_entity(u64::MAX)]
#[case::unknown_entity(2)]
fn health_delta_for_unresolvable_entity_is_skipped(#[case] health_entity: u64) {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_state(&mut app, entity);
    push_health_inputs_for(&mut app, health_entity, 90, 50);

    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .expect("applying DBSP outputs should succeed");

    let health = app
        .world()
        .entity(entity)
        .get::<Health>()
        .expect("Health component should remain after applying DBSP outputs");
    assert_eq!(
        health.current, 90,
        "an unresolvable health delta must not mutate the mapped entity's Health"
    );
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

#[rstest]
fn negative_weight_velocity_is_not_applied() {
    // Place the entity well above the floor (block at z=0) so it is
    // *unsupported* and the circuit integrates the velocity input into a
    // non-default velocity output — a standing entity would ignore the input,
    // making the gate assertion vacuous. The same velocity input drives both
    // phases so the Z-set weight is the only variable.
    let base = Position {
        entity: 1,
        x: 0.0.into(),
        y: 0.0.into(),
        z: 10.0.into(),
    };
    let velocity = Velocity {
        entity: 1,
        vx: 1.0.into(),
        vy: 2.0.into(),
        vz: 3.0.into(),
    };
    let default = VelocityComp::default();

    // The base position is always pushed at +1; only the velocity's weight
    // varies. `VelocityComp` derives no `PartialEq`, so the read copies its
    // components out as a comparable tuple.
    let push_inputs = |app: &mut App, weight: i64| {
        push_position_input(app, base, 1);
        push_velocity_input(app, velocity, weight);
    };
    let read_velocity = |app: &App, entity: Entity| {
        let applied = app.world().entity(entity).get::<VelocityComp>()?;
        Some((applied.vx, applied.vy, applied.vz))
    };
    let baseline = (default.vx, default.vy, default.vz);

    // Phase 1: at weight +1 the velocity output IS applied, so VelocityComp
    // changes from its default. This proves these inputs emit velocity output
    // at all, which keeps the phase-2 skip assertion from passing vacuously.
    let applied = run_weight_gate_phase(1, push_inputs, read_velocity)
        .expect("running the control phase should succeed")
        .expect("VelocityComp should remain present");
    assert_ne!(
        applied, baseline,
        "a positive-weight velocity must be applied, else the gate test is vacuous"
    );

    // Phase 2: the same inputs at weight -1. The negative (retraction) weight
    // must be skipped, leaving VelocityComp at its default.
    let skipped = run_weight_gate_phase(-1, push_inputs, read_velocity)
        .expect("running the retraction phase should succeed")
        .expect("VelocityComp should remain present");
    assert_eq!(
        skipped, baseline,
        "a negative-weight velocity must not mutate VelocityComp"
    );
}

#[rstest]
fn negative_weight_health_delta_is_not_applied() {
    let mut app = setup_app().expect("failed to set up test app");
    let entity = spawn_entity(&mut app);
    prime_entity_mapping(&mut app, entity);
    // Positive control: `applies_outputs_updates_components` primes these same
    // inputs — health 90/100 and 50 external damage at tick 1, seq 1 — with the
    // health snapshot at weight +1, and asserts `Health::current` moves 90 -> 40.
    // Here the snapshot is retracted instead (weight -1), so the Z-set weight is
    // the only variable: the consolidated health delta carries a negative
    // (retraction) weight and must be skipped rather than mutating Health.
    {
        let state = app.world_mut().non_send_resource_mut::<DbspState>();
        state.circuit.health_state_in().push(
            HealthState {
                entity: 1,
                current: 90,
                max: 100,
            },
            -1,
        );
        state.circuit.damage_in().push(
            DamageEvent {
                entity: 1,
                amount: 50,
                source: DamageSource::External,
                at_tick: 1,
                seq: Some(1),
            },
            1,
        );
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
        health.current, 90,
        "a negative-weight (retraction) health delta must not mutate Health"
    );
}
