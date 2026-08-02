//! Tests for the bounded reliability counters on [`DbspState`].
//!
//! `skipped_outputs()` tallies output records the weight gates discarded.
//! These tests pin the exact increment per gate; the sibling
//! [`super::failure_paths`] module covers `step_failures()`.

use super::*;
use std::io;

/// Pushes this case's inputs, applies the outputs, and returns how far
/// `skipped_outputs()` moved. Reading the counter either side makes the
/// assertion a delta rather than an absolute, so it stays valid if some other
/// path ever contributes to the same counter.
///
/// Returns the fallible `Result` rather than unwrapping, so the setup stays
/// outside a `no_expect_outside_tests` boundary; callers unwrap it.
fn skipped_outputs_delta(push_inputs: impl FnOnce(&mut App)) -> Result<u64, dbsp::Error> {
    let mut app = setup_app()?;
    let entity = spawn_entity(&mut app);
    prime_entity_mapping(&mut app, entity);
    let before = app
        .world()
        .non_send_resource::<DbspState>()
        .skipped_outputs();

    push_inputs(&mut app);
    // `RunSystemError` implements neither `Display` nor `Error`, so fold it
    // into the `dbsp::Error` this helper already returns, as
    // `run_weight_gate_phase` does.
    app.world_mut()
        .run_system_once(apply_dbsp_outputs_system)
        .map_err(|error| {
            dbsp::Error::IO(io::Error::other(format!(
                "applying DBSP outputs failed: {error:?}"
            )))
        })?;

    let after = app
        .world()
        .non_send_resource::<DbspState>()
        .skipped_outputs();
    Ok(after - before)
}

/// Retracting the health snapshot drives the health gate on its own: with no
/// position or velocity inputs those two outputs are empty, so exactly one
/// record is skipped. These are `applies_outputs_updates_components`' inputs
/// with the snapshot weight flipped, which that test proves move `Health`
/// 90 -> 40 at weight `+1`, so the count cannot be reached vacuously.
#[rstest]
fn retracted_health_delta_is_counted_as_skipped() {
    let skipped = skipped_outputs_delta(|app| {
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
    })
    .expect("applying DBSP outputs should succeed");

    assert_eq!(
        skipped, 1,
        "a retracted health delta must count as exactly one skipped record"
    );
}

/// The position and velocity gates cannot be driven independently: both
/// outputs derive from the same position/velocity join, so retracting either
/// input yields one negative-weight record on *each* handle, and retracting one
/// input without the other produces no output at all (verified empirically —
/// `velocity` alone and `position` alone both skip zero records). Each case
/// therefore asserts exactly two, which is still an exact count rather than the
/// `> 0` this replaced.
///
/// Both cases pair the retraction with a positive-weight partner that a
/// sibling test already proves produces applied output —
/// `negative_weight_position_is_not_applied`'s standing inputs, and
/// `negative_weight_velocity_is_not_applied`'s unsupported ones — so neither
/// count can be reached vacuously.
#[rstest]
#[case::retracted_position_standing(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 1.0.into() },
    -1,
    Velocity { entity: 1, vx: 1.0.into(), vy: 0.0.into(), vz: 0.0.into() },
    1
)]
#[case::retracted_velocity_unsupported(
    Position { entity: 1, x: 0.0.into(), y: 0.0.into(), z: 10.0.into() },
    1,
    Velocity { entity: 1, vx: 1.0.into(), vy: 2.0.into(), vz: 3.0.into() },
    -1
)]
fn retracted_motion_records_are_counted_as_skipped(
    #[case] position: Position,
    #[case] position_weight: i64,
    #[case] velocity: Velocity,
    #[case] velocity_weight: i64,
) {
    let skipped = skipped_outputs_delta(|app| {
        push_position_input(app, position, position_weight);
        push_velocity_input(app, velocity, velocity_weight);
    })
    .expect("applying DBSP outputs should succeed");

    assert_eq!(
        skipped, 2,
        "a retracted motion record must count one skipped position and one \
         skipped velocity"
    );
}
