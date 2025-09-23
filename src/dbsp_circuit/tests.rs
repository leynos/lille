//! Tests for DBSP circuit behaviour, including grace distance and health aggregation.
use super::*;
use crate::dbsp_circuit::streams::health_delta_stream;
use crate::dbsp_circuit::{DamageEvent, DamageSource, HealthDelta, HealthState};
use crate::GRACE_DISTANCE;
use dbsp::RootCircuit;
use rstest::rstest;

fn make_pf(z: f64, z_floor: f64) -> PositionFloor {
    PositionFloor {
        position: Position {
            entity: 1,
            x: 0.0.into(),
            y: 0.0.into(),
            z: z.into(),
        },
        z_floor: z_floor.into(),
    }
}

#[rstest]
#[case(10.0, 10.0)]
#[case(10.05, 10.0)]
#[case(10.1, 10.0)]
#[case(0.0, 0.0)]
#[case(-10.0, -10.0)]
#[case(-10.05, -10.0)]
#[case(f64::NAN, 10.0)]
#[case(10.0, f64::NAN)]
#[case(f64::NAN, f64::NAN)]
fn within_grace(#[case] z: f64, #[case] z_floor: f64) {
    let pf = make_pf(z, z_floor);
    if z.is_nan() || z_floor.is_nan() {
        // Comparisons with NaN are always false -> outside grace.
        let within = pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE;
        assert!(!within);
    } else {
        assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
    }
}

#[rstest]
#[case(11.0, 10.0)]
#[case(10.0 + GRACE_DISTANCE, 10.0)]
fn beyond_grace_or_at_boundary(#[case] z: f64, #[case] z_floor: f64) {
    let pf = make_pf(z, z_floor);
    if pf.position.z.into_inner() == pf.z_floor.into_inner() + GRACE_DISTANCE {
        assert!(pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE);
    } else {
        assert!(pf.position.z.into_inner() > pf.z_floor.into_inner() + GRACE_DISTANCE);
    }
}

fn run_health_delta(health: HealthState, events: &[(DamageEvent, i32)]) -> Vec<HealthDelta> {
    let (circuit, (health_handle, damage_handle, output)) = RootCircuit::build(|circuit| {
        let (health_stream, health_handle) = circuit.add_input_zset::<HealthState>();
        let (damage_stream, damage_handle) = circuit.add_input_zset::<DamageEvent>();
        let output = health_delta_stream(&health_stream, &damage_stream).output();
        Ok((health_handle, damage_handle, output))
    })
    .expect("failed to build health circuit");

    health_handle.push(health, 1);
    for (event, weight) in events {
        damage_handle.push(*event, i64::from(*weight));
    }

    circuit.step().expect("health circuit step failed");
    output
        .consolidate()
        .iter()
        .map(|(delta, _, _)| delta)
        .collect()
}

fn assert_health_delta_test(
    health_state: HealthState,
    events: &[(DamageEvent, i32)],
    expected_delta: i32,
    expected_death: bool,
    expected_seq: Option<u32>,
) {
    let deltas = run_health_delta(health_state, events);
    assert_eq!(deltas.len(), 1);
    let delta = deltas[0];
    assert_eq!(delta.delta, expected_delta);
    assert_eq!(delta.death, expected_death);
    assert_eq!(delta.seq, expected_seq);
}

fn run_health_delta_test(
    entity: u64,
    current: u16,
    max: u16,
    events: Vec<(u16, DamageSource, u64, Option<u32>)>,
    expected_delta: i32,
    expected_death: bool,
    expected_seq: Option<u32>,
) {
    let health = HealthState {
        entity,
        current,
        max,
    };
    let damage_events: Vec<(DamageEvent, i32)> = events
        .into_iter()
        .map(|(amount, source, at_tick, seq)| {
            (
                DamageEvent {
                    entity,
                    amount,
                    source,
                    at_tick,
                    seq,
                },
                1,
            )
        })
        .collect();

    assert_health_delta_test(
        health,
        &damage_events,
        expected_delta,
        expected_death,
        expected_seq,
    );
}

#[rstest]
#[case(80, 100, 50, 20)]
#[case(10, 50, 5, 5)]
fn healing_clamped_to_max(
    #[case] current: u16,
    #[case] max: u16,
    #[case] heal: u16,
    #[case] expected_delta: i32,
) {
    let health = HealthState {
        entity: 1,
        current,
        max,
    };
    let event = DamageEvent {
        entity: 1,
        amount: heal,
        source: DamageSource::Script,
        at_tick: 5,
        seq: Some(1),
    };

    let deltas = run_health_delta(health, &[(event, 1)]);
    assert_eq!(deltas.len(), 1);
    let delta = deltas[0];
    assert_eq!(delta.delta, expected_delta);
    assert!(!delta.death);
    assert_eq!(delta.seq, Some(1));
}

#[rstest]
#[case(Some(3))]
#[case(None)]
fn duplicate_damage_events_idempotent(#[case] seq: Option<u32>) {
    let health = HealthState {
        entity: 2,
        current: 90,
        max: 100,
    };
    let event = DamageEvent {
        entity: 2,
        amount: 30,
        source: DamageSource::External,
        at_tick: 9,
        seq,
    };

    assert_health_delta_test(health, &[(event, 1), (event, 1)], -30, false, seq);
}

#[rstest]
fn sequenced_events_with_same_seq_in_same_tick_are_deduplicated() {
    let health = HealthState {
        entity: 7,
        current: 70,
        max: 100,
    };
    let first = DamageEvent {
        entity: 7,
        amount: 20,
        source: DamageSource::External,
        at_tick: 8,
        seq: Some(11),
    };
    // Provide the duplicate event with an identical payload to mirror the ingress
    // first-write-wins policy: later `(entity, tick, seq)` writes are ignored, and
    // the matching payload ensures the circuit's debug assertions are satisfied.
    let second = DamageEvent {
        entity: 7,
        amount: 20,
        source: DamageSource::External,
        at_tick: 8,
        seq: Some(11),
    };

    assert_health_delta_test(health, &[(first, 1), (second, 1)], -20, false, Some(11));
}

#[rstest]
#[case::unsequenced_distinct_sources(6, 40, 100, vec![(15, DamageSource::External, 4, None), (25, DamageSource::Script, 4, None)], 10, false, None)]
#[case::unsequenced_duplicate_payloads_filtered(6, 40, 100, vec![(15, DamageSource::External, 4, None), (15, DamageSource::External, 4, None)], -15, false, None)]
#[case::multiple_events_max_seq(5, 100, 120, vec![(60, DamageSource::External, 10, Some(1)), (20, DamageSource::Script, 10, Some(4))], -40, false, Some(4))]
#[case::healing_from_zero(4, 0, 80, vec![(30, DamageSource::Script, 3, None)], 30, false, None)]
#[case::over_healing_clamped(5, 0, 80, vec![(150, DamageSource::Script, 4, None)], 80, false, None)]
fn health_delta_scenarios(
    #[case] entity: u64,
    #[case] current: u16,
    #[case] max: u16,
    #[case] events: Vec<(u16, DamageSource, u64, Option<u32>)>,
    #[case] expected_delta: i32,
    #[case] expected_death: bool,
    #[case] expected_seq: Option<u32>,
) {
    run_health_delta_test(
        entity,
        current,
        max,
        events,
        expected_delta,
        expected_death,
        expected_seq,
    );
}

#[rstest]
fn lethal_damage_sets_death_flag() {
    run_health_delta_test(
        3,
        20,
        50,
        vec![(40, DamageSource::External, 2, Some(7))],
        -20,
        true,
        Some(7),
    );
}
