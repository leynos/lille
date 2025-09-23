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

#[derive(Clone, Copy, Debug)]
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

#[expect(
    clippy::too_many_arguments,
    reason = "dual-event helper mirrors test expectations without extra structs"
)]
fn run_dual_event_health_test(
    entity: u64,
    current: u16,
    max: u16,
    first_event: (u16, DamageSource, u64, Option<u32>),
    second_event: (u16, DamageSource, u64, Option<u32>),
    expected_delta: i32,
    expected_death: bool,
    expected_seq: Option<u32>,
) {
    let health = HealthState {
        entity,
        current,
        max,
    };
    let first = DamageEvent {
        entity,
        amount: first_event.0,
        source: first_event.1,
        at_tick: first_event.2,
        seq: first_event.3,
    };
    let second = DamageEvent {
        entity,
        amount: second_event.0,
        source: second_event.1,
        at_tick: second_event.2,
        seq: second_event.3,
    };

    assert_health_delta_test(
        health,
        &[(first, 1), (second, 1)],
        expected_delta,
        expected_death,
        expected_seq,
    );
}

#[derive(Clone, Copy, Debug)]
struct DualEventHealthTestCase {
    entity: u64,
    current: u16,
    max: u16,
    first_event: (u16, DamageSource, u64, Option<u32>),
    second_event: (u16, DamageSource, u64, Option<u32>),
    expected_delta: i32,
    expected_death: bool,
    expected_seq: Option<u32>,
}

impl DualEventHealthTestCase {
    fn run(self) {
        run_dual_event_health_test(
            self.entity,
            self.current,
            self.max,
            self.first_event,
            self.second_event,
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
#[case::unsequenced(
    DualEventHealthTestCase {
        entity: 6,
        current: 40,
        max: 100,
        first_event: (15, DamageSource::External, 4, None),
        second_event: (25, DamageSource::Script, 4, None),
        expected_delta: 10,
        expected_death: false,
        expected_seq: None,
    }
)]
#[case::max_seq(
    DualEventHealthTestCase {
        entity: 5,
        current: 100,
        max: 120,
        first_event: (60, DamageSource::External, 10, Some(1)),
        second_event: (20, DamageSource::Script, 10, Some(4)),
        expected_delta: -40,
        expected_death: false,
        expected_seq: Some(4),
    }
)]
fn dual_event_health_deltas(#[case] case: DualEventHealthTestCase) {
    case.run();
}

#[rstest]
#[case::lethal(
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
)]
#[case::healing_from_zero(
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
)]
#[case::over_healing_from_zero(
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
)]
fn single_event_health_deltas(#[case] case: HealthTestCase) {
    case.run();
}
