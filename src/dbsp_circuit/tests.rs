//! Tests for DBSP circuit behaviour, including grace distance and health aggregation.
use super::*;
use crate::dbsp_circuit::streams::health_delta_stream;
use crate::dbsp_circuit::{DamageEvent, DamageSource, HealthDelta, HealthState};
use crate::GRACE_DISTANCE;
use approx::abs_diff_eq;
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
    let position_z = pf.position.z.into_inner();
    let boundary = pf.z_floor.into_inner() + GRACE_DISTANCE;
    let tol = 1e-12;
    if abs_diff_eq!(position_z, boundary, epsilon = tol) {
        assert!(position_z <= boundary + tol);
    } else {
        assert!(position_z > boundary + tol);
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
        .map(|(delta, (), _)| delta)
        .collect()
}

/// Specifies a damage event used by health delta tests, capturing the amount,
/// source, tick, and optional sequence identifier so `DamageEvent` records can
/// be constructed consistently in scenarios.
#[derive(Clone, Copy)]
struct DamageEventSpec {
    amount: u16,
    source: DamageSource,
    at_tick: u64,
    seq: Option<u32>,
}

impl DamageEventSpec {
    const fn new(amount: u16, source: DamageSource, at_tick: u64, seq: Option<u32>) -> Self {
        Self {
            amount,
            source,
            at_tick,
            seq,
        }
    }
}

/// Bundles an initial health state with a sequence of damage events to
/// validate the resulting health delta outputs produced by the circuit.
struct HealthDeltaTestCase {
    state: HealthState,
    events: Vec<DamageEventSpec>,
}

impl HealthDeltaTestCase {
    /// Create a new health delta test case for `entity` with the provided
    /// current/max health and damage event specifications.
    fn new(entity: u64, current: u16, max: u16, events: Vec<DamageEventSpec>) -> Self {
        Self {
            state: HealthState {
                entity,
                current,
                max,
            },
            events,
        }
    }

    /// Convert the damage specs into weighted `DamageEvent` records for circuit input.
    fn event_records(&self) -> Vec<(DamageEvent, i32)> {
        self.events
            .iter()
            .map(|spec| {
                (
                    DamageEvent {
                        entity: self.state.entity,
                        amount: spec.amount,
                        source: spec.source,
                        at_tick: spec.at_tick,
                        seq: spec.seq,
                    },
                    1,
                )
            })
            .collect()
    }
}

/// Expected outcome for a health delta scenario, including the delta amount,
/// whether the entity should be dead afterwards, and an optional sequence id to match against.
#[derive(Clone, Copy)]
struct HealthDeltaExpectation {
    delta: i32,
    death: bool,
    seq: Option<u32>,
}

/// Runs the health delta circuit for a test case and asserts the single emitted
/// [`HealthDelta`] matches the expected delta, death flag, and optional
/// sequence id, panicking if the circuit produces anything other than one result.
fn assert_health_delta(case: &HealthDeltaTestCase, expected: HealthDeltaExpectation) {
    let events = case.event_records();
    let deltas = run_health_delta(case.state, &events);
    match deltas.as_slice() {
        [delta] => {
            assert_eq!(delta.delta, expected.delta);
            assert_eq!(delta.death, expected.death);
            assert_eq!(delta.seq, expected.seq);
        }
        _ => panic!("expected exactly one health delta, found {}", deltas.len()),
    }
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
    let case = HealthDeltaTestCase::new(
        1,
        current,
        max,
        vec![DamageEventSpec::new(heal, DamageSource::Script, 5, Some(1))],
    );
    assert_health_delta(
        &case,
        HealthDeltaExpectation {
            delta: expected_delta,
            death: false,
            seq: Some(1),
        },
    );
}

#[rstest]
#[case(Some(3))]
#[case(None)]
fn duplicate_damage_events_idempotent(#[case] seq: Option<u32>) {
    let event = DamageEventSpec::new(30, DamageSource::External, 9, seq);
    let case = HealthDeltaTestCase::new(2, 90, 100, vec![event, event]);
    assert_health_delta(
        &case,
        HealthDeltaExpectation {
            delta: -30,
            death: false,
            seq,
        },
    );
}

#[rstest]
fn sequenced_events_with_same_seq_in_same_tick_are_deduplicated() {
    let event = DamageEventSpec::new(20, DamageSource::External, 8, Some(11));
    // Provide the duplicate event with an identical payload to mirror the ingress
    // first-write-wins policy: later `(entity, tick, seq)` writes are ignored, and
    // the matching payload ensures the circuit's debug assertions are satisfied.
    let case = HealthDeltaTestCase::new(7, 70, 100, vec![event, event]);
    assert_health_delta(
        &case,
        HealthDeltaExpectation {
            delta: -20,
            death: false,
            seq: Some(11),
        },
    );
}

#[rstest]
#[case::unsequenced_distinct_sources(
    HealthDeltaTestCase::new(
        6,
        40,
        100,
        vec![
            DamageEventSpec::new(15, DamageSource::External, 4, None),
            DamageEventSpec::new(25, DamageSource::Script, 4, None),
        ],
    ),
    HealthDeltaExpectation {
        delta: 10,
        death: false,
        seq: None,
    }
)]
#[case::unsequenced_duplicate_payloads_filtered(
    HealthDeltaTestCase::new(
        6,
        40,
        100,
        vec![
            DamageEventSpec::new(15, DamageSource::External, 4, None),
            DamageEventSpec::new(15, DamageSource::External, 4, None),
        ],
    ),
    HealthDeltaExpectation {
        delta: -15,
        death: false,
        seq: None,
    }
)]
#[case::multiple_events_max_seq(
    HealthDeltaTestCase::new(
        5,
        100,
        120,
        vec![
            DamageEventSpec::new(60, DamageSource::External, 10, Some(1)),
            DamageEventSpec::new(20, DamageSource::Script, 10, Some(4)),
        ],
    ),
    HealthDeltaExpectation {
        delta: -40,
        death: false,
        seq: Some(4),
    }
)]
#[case::healing_from_zero(
    HealthDeltaTestCase::new(
        4,
        0,
        80,
        vec![DamageEventSpec::new(30, DamageSource::Script, 3, None)],
    ),
    HealthDeltaExpectation {
        delta: 30,
        death: false,
        seq: None,
    }
)]
#[case::over_healing_clamped(
    HealthDeltaTestCase::new(
        5,
        0,
        80,
        vec![DamageEventSpec::new(150, DamageSource::Script, 4, None)],
    ),
    HealthDeltaExpectation {
        delta: 80,
        death: false,
        seq: None,
    }
)]
fn health_delta_scenarios(
    #[case] case: HealthDeltaTestCase,
    #[case] expected: HealthDeltaExpectation,
) {
    assert_health_delta(&case, expected);
}

#[derive(Clone, Copy)]
struct LethalCase {
    entity: u64,
    current: u16,
    max: u16,
    damage_amount: u16,
    expected_delta: i32,
    seq: Option<u32>,
}

#[rstest]
#[case::moderate_damage(LethalCase {
    entity: 3,
    current: 10,
    max: 100,
    damage_amount: 15,
    expected_delta: -10,
    seq: Some(2),
})]
#[case::high_damage(LethalCase {
    entity: 3,
    current: 20,
    max: 50,
    damage_amount: 40,
    expected_delta: -20,
    seq: Some(7),
})]
fn lethal_damage_sets_death_flag(#[case] params: LethalCase) {
    const TICK: u64 = 2;
    let LethalCase {
        entity,
        current,
        max,
        damage_amount,
        expected_delta,
        seq,
    } = params;
    let case = HealthDeltaTestCase::new(
        entity,
        current,
        max,
        vec![DamageEventSpec::new(
            damage_amount,
            DamageSource::External,
            TICK,
            seq,
        )],
    );
    assert_health_delta(
        &case,
        HealthDeltaExpectation {
            delta: expected_delta,
            death: true,
            seq,
        },
    );
}
