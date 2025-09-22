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

struct HealthTestCase {
    entity: u32,
    current: u16,
    max: u16,
    damage_amount: u16,
    damage_source: DamageSource,
    at_tick: u32,
    seq: Option<u32>,
    expected_delta: i32,
    expected_death: bool,
    expected_seq: Option<u32>,
}

impl HealthTestCase {
    fn run(self) {
        let entity_id = u64::from(self.entity);
        let health = HealthState {
            entity: entity_id,
            current: self.current,
            max: self.max,
        };
        let event = DamageEvent {
            entity: entity_id,
            amount: self.damage_amount,
            source: self.damage_source,
            at_tick: u64::from(self.at_tick),
            seq: self.seq,
        };
        assert_health_delta_test(
            health,
            &[(event, 1)],
            self.expected_delta,
            self.expected_death,
            self.expected_seq,
        );
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
    // Provide the duplicate event with identical payload so debug assertions
    // enforce the idempotency contract.
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
fn unsequenced_events_with_distinct_sources_accumulate() {
    let health = HealthState {
        entity: 6,
        current: 40,
        max: 100,
    };
    let damage = DamageEvent {
        entity: 6,
        amount: 15,
        source: DamageSource::External,
        at_tick: 4,
        seq: None,
    };
    let heal = DamageEvent {
        entity: 6,
        amount: 25,
        source: DamageSource::Script,
        at_tick: 4,
        seq: None,
    };

    assert_health_delta_test(health, &[(damage, 1), (heal, 1)], 10, false, None);
}

#[rstest]
fn lethal_damage_sets_death_flag() {
    HealthTestCase {
        entity: 3,
        current: 20,
        max: 50,
        damage_amount: 40,
        damage_source: DamageSource::External,
        at_tick: 2,
        seq: Some(7),
        expected_delta: -20,
        expected_death: true,
        expected_seq: Some(7),
    }
    .run();
}

#[rstest]
fn healing_from_zero_produces_positive_delta() {
    HealthTestCase {
        entity: 4,
        current: 0,
        max: 80,
        damage_amount: 30,
        damage_source: DamageSource::Script,
        at_tick: 3,
        seq: None,
        expected_delta: 30,
        expected_death: false,
        expected_seq: None,
    }
    .run();
}

#[rstest]
fn over_healing_from_zero_is_clamped_to_max() {
    HealthTestCase {
        entity: 5,
        current: 0,
        max: 80,
        damage_amount: 150,
        damage_source: DamageSource::Script,
        at_tick: 4,
        seq: None,
        expected_delta: 80,
        expected_death: false,
        expected_seq: None,
    }
    .run();
}

#[rstest]
fn multiple_events_same_tick_accumulate_and_pick_max_seq() {
    let health = HealthState {
        entity: 5,
        current: 100,
        max: 120,
    };
    let damage = DamageEvent {
        entity: 5,
        amount: 60,
        source: DamageSource::External,
        at_tick: 10,
        seq: Some(1),
    };
    let heal = DamageEvent {
        entity: 5,
        amount: 20,
        source: DamageSource::Script,
        at_tick: 10,
        seq: Some(4),
    };
    assert_health_delta_test(health, &[(damage, 1), (heal, 1)], -40, false, Some(4));
}
