//! Tests for the health stream pipelines.

use super::fall_damage_stream;
use crate::dbsp_circuit::Position;
use crate::dbsp_circuit::{DamageEvent, DamageSource, PositionFloor, Tick, Velocity};
use crate::numeric::expect_u16;
use crate::{FALL_DAMAGE_SCALE, LANDING_COOLDOWN_TICKS, SAFE_LANDING_SPEED, TERMINAL_VELOCITY};
use dbsp::{operator::Generator, typed_batch::OrdZSet, Circuit, RootCircuit};
use ordered_float::OrderedFloat;
use rstest::rstest;
use std::collections::BTreeMap;

type FallDamageHarness = (
    dbsp::CircuitHandle,
    dbsp::ZSetHandle<PositionFloor>,
    dbsp::ZSetHandle<PositionFloor>,
    dbsp::ZSetHandle<Velocity>,
    dbsp::OutputHandle<OrdZSet<DamageEvent>>,
);

fn pf(entity: i64, z: f64, floor: f64) -> PositionFloor {
    PositionFloor {
        position: Position {
            entity,
            x: OrderedFloat(0.0),
            y: OrderedFloat(0.0),
            z: OrderedFloat(z),
        },
        z_floor: OrderedFloat(floor),
    }
}

fn vel(entity: i64, vz: f64) -> Velocity {
    Velocity {
        entity,
        vx: OrderedFloat(0.0),
        vy: OrderedFloat(0.0),
        vz: OrderedFloat(vz),
    }
}

fn build_circuit() -> FallDamageHarness {
    let (circuit, (standing_in, unsupported_in, velocity_in, output)) =
        RootCircuit::build(|circuit| {
            let (standing_stream, standing_in) = circuit.add_input_zset::<PositionFloor>();
            let (unsupported_stream, unsupported_in) = circuit.add_input_zset::<PositionFloor>();
            let (velocity_stream, velocity_in) = circuit.add_input_zset::<Velocity>();
            let tick_source = circuit.add_source(Generator::new({
                let mut tick: Tick = 0;
                move || {
                    let current = tick;
                    tick = tick.checked_add(1).expect("tick counter overflowed u64");
                    current
                }
            }));
            let current_tick = tick_source;
            let fall_damage = fall_damage_stream(
                &standing_stream,
                &unsupported_stream,
                &velocity_stream,
                &current_tick,
            );
            Ok((
                standing_in,
                unsupported_in,
                velocity_in,
                fall_damage.output(),
            ))
        })
        .expect("build fall damage circuit");

    (circuit, standing_in, unsupported_in, velocity_in, output)
}

fn read_events(output: &dbsp::OutputHandle<OrdZSet<DamageEvent>>) -> Vec<DamageEvent> {
    output
        .consolidate()
        .iter()
        .map(|(event, (), weight)| {
            assert_eq!(weight, 1, "expected single-weight damage events");
            event
        })
        .collect()
}

fn delta_events(
    output: &dbsp::OutputHandle<OrdZSet<DamageEvent>>,
    cumulative: &mut BTreeMap<DamageEvent, i64>,
) -> Vec<(DamageEvent, i64)> {
    let mut deltas = Vec::new();

    for (event, (), weight) in output.consolidate().iter() {
        if weight <= 0 {
            continue;
        }

        let entry = cumulative.entry(event).or_insert(0);
        if *entry == 0 {
            *entry = weight.signum();
            deltas.push((event, weight.signum()));
        }
    }

    deltas
}

#[rstest]
fn fall_damage_emits_event() {
    let (circuit, standing_in, unsupported_in, velocity_in, output) = build_circuit();

    let unsupported_pf = pf(1, 5.0, 0.0);
    let standing_pf = pf(1, 1.0, 1.0);
    let falling_vel = vel(1, -8.0);

    unsupported_in.push(unsupported_pf.clone(), 1);
    velocity_in.push(falling_vel, 1);
    circuit.step().expect("step unsupported phase");
    assert!(read_events(&output).is_empty());

    unsupported_in.push(unsupported_pf.clone(), -1);
    standing_in.push(standing_pf.clone(), 1);
    circuit.step().expect("step landing phase");

    let events = read_events(&output);
    assert_eq!(events.len(), 1);
    let event = events.first().expect("expected single fall damage event");
    assert_eq!(event.entity, 1);
    assert_eq!(event.source, DamageSource::Fall);
    let expected_amount_raw = ((8.0_f64.min(TERMINAL_VELOCITY) - SAFE_LANDING_SPEED)
        * FALL_DAMAGE_SCALE)
        .min(f64::from(u16::MAX))
        .floor();
    let expected_amount = expect_u16(expected_amount_raw);
    assert_eq!(event.amount, expected_amount);
    assert_eq!(event.at_tick, 1);
}

#[rstest]
fn multiple_entities_land_without_interference() {
    let (circuit, standing_in, unsupported_in, velocity_in, output) = build_circuit();

    let unsupported_pf_a = pf(1, 5.0, 0.0);
    let unsupported_pf_b = pf(2, 8.0, 0.0);
    let standing_pf_a = pf(1, 1.0, 1.0);
    let standing_pf_b = pf(2, 1.0, 1.0);
    let falling_vel_a = vel(1, -8.0);
    let falling_vel_b = vel(2, -12.0);

    unsupported_in.push(unsupported_pf_a.clone(), 1);
    unsupported_in.push(unsupported_pf_b.clone(), 1);
    velocity_in.push(falling_vel_a, 1);
    velocity_in.push(falling_vel_b, 1);
    circuit.step().expect("step unsupported phase");
    assert!(read_events(&output).is_empty());

    unsupported_in.push(unsupported_pf_a.clone(), -1);
    unsupported_in.push(unsupported_pf_b.clone(), -1);
    standing_in.push(standing_pf_a.clone(), 1);
    standing_in.push(standing_pf_b.clone(), 1);
    circuit.step().expect("step landing phase");

    let mut events = read_events(&output);
    events.sort_by_key(|event| event.entity);
    assert_eq!(events.len(), 2);

    let expected_a = ((8.0_f64.min(TERMINAL_VELOCITY) - SAFE_LANDING_SPEED) * FALL_DAMAGE_SCALE)
        .min(f64::from(u16::MAX))
        .floor();
    let expected_b = ((12.0_f64.min(TERMINAL_VELOCITY) - SAFE_LANDING_SPEED) * FALL_DAMAGE_SCALE)
        .min(f64::from(u16::MAX))
        .floor();

    let first = events.first().expect("expected first fall damage event");
    assert_eq!(first.entity, 1);
    assert_eq!(first.source, DamageSource::Fall);
    assert_eq!(first.amount, expect_u16(expected_a));
    assert_eq!(first.at_tick, 1);

    let second = events.get(1).expect("expected second fall damage event");
    assert_eq!(second.entity, 2);
    assert_eq!(second.source, DamageSource::Fall);
    assert_eq!(second.amount, expect_u16(expected_b));
    assert_eq!(second.at_tick, 1);
}

#[rstest]
fn safe_speed_emits_no_damage() {
    let (circuit, standing_in, unsupported_in, velocity_in, output) = build_circuit();
    let unsupported_pf = pf(2, 5.0, 0.0);
    let standing_pf = pf(2, 1.0, 1.0);
    let falling_vel = vel(2, -4.0);

    unsupported_in.push(unsupported_pf.clone(), 1);
    velocity_in.push(falling_vel, 1);
    circuit.step().expect("unsupported tick");

    unsupported_in.push(unsupported_pf.clone(), -1);
    standing_in.push(standing_pf.clone(), 1);
    circuit.step().expect("landing tick");

    assert!(read_events(&output).is_empty());
}

#[rstest]
fn cooldown_prevents_rapid_retrigger() {
    let (circuit, standing_in, unsupported_in, velocity_in, output) = build_circuit();
    let unsupported_pf = pf(3, 5.0, 0.0);
    let standing_pf = pf(3, 1.0, 1.0);
    let falling_vel = vel(3, -9.0);
    let mut cumulative = BTreeMap::new();

    unsupported_in.push(unsupported_pf.clone(), 1);
    velocity_in.push(falling_vel, 1);
    circuit.step().expect("initial fall");

    unsupported_in.push(unsupported_pf.clone(), -1);
    standing_in.push(standing_pf.clone(), 1);
    circuit.step().expect("initial landing");
    let initial_events = delta_events(&output, &mut cumulative);
    assert_eq!(initial_events.len(), 1);
    let &(ref first_event_record, first_count) = initial_events
        .first()
        .expect("expected initial landing event");
    assert_eq!(first_count, 1);

    standing_in.push(standing_pf.clone(), -1);
    unsupported_in.push(unsupported_pf.clone(), 1);
    velocity_in.push(falling_vel, 1);
    circuit.step().expect("second fall");

    unsupported_in.push(unsupported_pf.clone(), -1);
    standing_in.push(standing_pf.clone(), 1);
    circuit.step().expect("second landing within cooldown");
    let cooldown_events = delta_events(&output, &mut cumulative);
    assert!(cooldown_events.is_empty());

    standing_in.push(standing_pf.clone(), -1);
    for _ in 0..LANDING_COOLDOWN_TICKS {
        circuit.step().expect("cooldown tick");
    }

    unsupported_in.push(unsupported_pf.clone(), 1);
    velocity_in.push(falling_vel, 1);
    circuit.step().expect("post-cooldown fall");

    unsupported_in.push(unsupported_pf.clone(), -1);
    standing_in.push(standing_pf.clone(), 1);
    circuit.step().expect("post-cooldown landing");
    let final_events = delta_events(&output, &mut cumulative);
    assert_eq!(final_events.len(), 1);
    let &(ref final_event_record, final_count) =
        final_events.first().expect("expected final landing event");
    assert_eq!(final_count, 1);

    assert!(final_event_record.at_tick > first_event_record.at_tick);
    assert_eq!(cumulative.len(), 2);
}
