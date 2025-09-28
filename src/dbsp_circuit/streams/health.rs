//! Health aggregation streams.
//!
//! These helpers reduce health snapshots and incoming damage events to
//! authoritative [`HealthDelta`] records emitted by the DBSP circuit.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::dbsp_circuit::{
    DamageEvent, DamageSource, HealthDelta, HealthState, PositionFloor, Tick, Velocity,
};
use crate::{FALL_DAMAGE_SCALE, LANDING_COOLDOWN_TICKS, SAFE_LANDING_SPEED, TERMINAL_VELOCITY};
use dbsp::utils::Tup2;
use dbsp::{algebra::Semigroup, operator::Fold, typed_batch::OrdZSet, RootCircuit, Stream};
use ordered_float::OrderedFloat;

#[derive(
    ::rkyv::Archive,
    ::rkyv::Serialize,
    ::rkyv::Deserialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    ::size_of::SizeOf,
)]
#[archive_attr(derive(Eq, PartialEq, Ord, PartialOrd, Hash))]
struct HealthAccumulator {
    sequenced: BTreeMap<u32, i32>,
    // BTreeSet keeps iteration deterministic while deduplicating identical
    // payloads, satisfying DBSP's archive stability requirements.
    unsequenced: BTreeSet<DamageEvent>,
    has_event: bool,
}

impl HealthAccumulator {
    fn insert(&mut self, event: &DamageEvent) {
        self.has_event = true;
        if let Some(seq) = event.seq {
            let signed = signed_amount(event);
            match self.sequenced.entry(seq) {
                Entry::Vacant(slot) => {
                    slot.insert(signed);
                }
                Entry::Occupied(existing) => {
                    let existing_signed = *existing.get();
                    debug_assert_eq!(
                        existing_signed,
                        signed,
                        "sequenced damage event mismatch for seq {seq}: existing {existing_signed}, incoming {signed}",
                        seq = seq,
                        existing_signed = existing_signed,
                        signed = signed,
                    );
                }
            }
        } else {
            self.unsequenced.insert(*event);
        }
    }

    fn remove(&mut self, event: &DamageEvent) {
        if let Some(seq) = event.seq {
            self.sequenced.remove(&seq);
        } else {
            self.unsequenced.remove(event);
        }
        self.has_event = !self.sequenced.is_empty() || !self.unsequenced.is_empty();
    }

    fn merge(&mut self, other: &Self) {
        self.merge_sequenced_events(&other.sequenced);
        self.merge_unsequenced_events(&other.unsequenced);
        self.has_event = !self.sequenced.is_empty() || !self.unsequenced.is_empty();
    }

    fn merge_sequenced_events(&mut self, sequenced: &BTreeMap<u32, i32>) {
        for (seq, signed) in sequenced {
            let seq_value = *seq;
            let incoming_signed = *signed;
            match self.sequenced.entry(seq_value) {
                Entry::Vacant(slot) => {
                    slot.insert(incoming_signed);
                }
                Entry::Occupied(existing) => {
                    let existing_signed = *existing.get();
                    debug_assert_eq!(
                        existing_signed,
                        incoming_signed,
                        "sequenced damage event mismatch for seq {seq}: existing {existing_signed}, incoming {incoming_signed}",
                        seq = seq_value,
                        existing_signed = existing_signed,
                        incoming_signed = incoming_signed,
                    );
                }
            }
        }
    }

    fn merge_unsequenced_events(&mut self, unsequenced: &BTreeSet<DamageEvent>) {
        self.unsequenced.extend(unsequenced.iter().copied());
    }
}

#[derive(Clone)]
struct HealthAccumulatorSemigroup;

impl Semigroup<HealthAccumulator> for HealthAccumulatorSemigroup {
    fn combine(left: &HealthAccumulator, right: &HealthAccumulator) -> HealthAccumulator {
        let mut combined = left.clone();
        combined.merge(right);
        combined
    }
}

#[derive(
    ::rkyv::Archive,
    ::rkyv::Serialize,
    ::rkyv::Deserialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    ::size_of::SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
struct HealthAggregate {
    net: i32,
    max_seq: Option<u32>,
    has_event: bool,
}

fn signed_amount(event: &DamageEvent) -> i32 {
    match event.source {
        DamageSource::Script => i32::from(event.amount),
        DamageSource::External | DamageSource::Fall | DamageSource::Other(_) => {
            -i32::from(event.amount)
        }
    }
}

/// Derives fall damage events from landing transitions.
fn detect_landings(
    standing: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    unsupported: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
) -> Stream<RootCircuit, OrdZSet<i64>> {
    let standing_entities = standing.map(|pf| pf.position.entity);
    let prev_unsupported = unsupported.map(|pf| pf.position.entity).delay();

    prev_unsupported.map_index(|entity| (*entity, ())).join(
        &standing_entities.map_index(|entity| (*entity, ())),
        |entity, _, _| *entity,
    )
}

fn apply_landing_cooldown(
    landings: &Stream<RootCircuit, OrdZSet<i64>>,
) -> Stream<RootCircuit, OrdZSet<i64>> {
    let mut cooldown_end = landings.clone();
    for _ in 0..LANDING_COOLDOWN_TICKS {
        cooldown_end = cooldown_end.delay();
    }

    let cooldown_updates = landings.clone().plus(&cooldown_end.neg());
    let active_cooldown = cooldown_updates.integrate();
    let cooling_entities = active_cooldown.delay().map_index(|entity| (*entity, ()));

    landings
        .map_index(|entity| (*entity, ()))
        .antijoin(&cooling_entities)
        .map(|(entity, _)| *entity)
}

fn calculate_fall_damage(
    allowed_landings: &Stream<RootCircuit, OrdZSet<i64>>,
    unsupported_velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
    ticks: &Stream<RootCircuit, Tick>,
) -> Stream<RootCircuit, OrdZSet<DamageEvent>> {
    let prev_velocities = unsupported_velocities.delay();
    let landing_impacts = allowed_landings
        .map_index(|entity| (*entity, *entity))
        .join(
            &prev_velocities.map_index(|vel| (vel.entity, vel.vz)),
            |_entity, &landing_entity, &vz| (landing_entity, vz),
        );

    let downward_impacts = landing_impacts.flat_map(|&(entity, vz)| {
        let speed = -vz.into_inner();
        (speed > 0.0)
            .then_some((entity, OrderedFloat(speed)))
            .into_iter()
    });

    downward_impacts.apply2(ticks, |impacts, tick| {
        let mut tuples = Vec::new();
        for ((entity, speed), (), weight) in impacts.iter() {
            if weight == 0 {
                continue;
            }
            let entity_id = match u64::try_from(entity) {
                Ok(id) => id,
                Err(_) => {
                    debug_assert!(false, "negative entity id {entity}");
                    continue;
                }
            };
            let clamped_speed = speed.into_inner().min(TERMINAL_VELOCITY);
            let excess = clamped_speed - SAFE_LANDING_SPEED;
            if excess <= 0.0 {
                continue;
            }
            let scaled = excess * FALL_DAMAGE_SCALE;
            if scaled <= 0.0 {
                continue;
            }
            let damage = scaled.min(f64::from(u16::MAX)).floor() as u16;
            if damage == 0 {
                continue;
            }
            let event = DamageEvent {
                entity: entity_id,
                amount: damage,
                source: DamageSource::Fall,
                at_tick: *tick,
                seq: None,
            };
            tuples.push(Tup2(Tup2(event, ()), weight));
        }
        OrdZSet::from_tuples((), tuples)
    })
}

/// Derives fall damage events from landing transitions.
pub fn fall_damage_stream(
    standing: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    unsupported: &Stream<RootCircuit, OrdZSet<PositionFloor>>,
    unsupported_velocities: &Stream<RootCircuit, OrdZSet<Velocity>>,
    ticks: &Stream<RootCircuit, Tick>,
) -> Stream<RootCircuit, OrdZSet<DamageEvent>> {
    let landings = detect_landings(standing, unsupported);
    let allowed_landings = apply_landing_cooldown(&landings);
    calculate_fall_damage(&allowed_landings, unsupported_velocities, ticks)
}

pub fn health_delta_stream(
    health_states: &Stream<RootCircuit, OrdZSet<HealthState>>,
    damage_events: &Stream<RootCircuit, OrdZSet<DamageEvent>>,
) -> Stream<RootCircuit, OrdZSet<HealthDelta>> {
    let health_indexed = health_states.map_index(|state| (state.entity, *state));

    let aggregated = damage_events
        .map_index(|event| ((event.entity, event.at_tick), *event))
        .aggregate(Fold::<
            DamageEvent,
            HealthAccumulator,
            HealthAccumulatorSemigroup,
            _,
            _,
        >::with_output(
            HealthAccumulator::default(),
            |acc: &mut HealthAccumulator, event: &DamageEvent, weight: i64| {
                if weight > 0 {
                    acc.insert(event);
                } else if weight < 0 {
                    acc.remove(event);
                }
            },
            |acc: HealthAccumulator| {
                let net_seq: i32 = acc.sequenced.values().copied().sum();
                let net_unseq: i32 = acc.unsequenced.iter().map(signed_amount).sum();
                let max_seq = acc.sequenced.keys().next_back().copied();
                HealthAggregate {
                    net: net_seq + net_unseq,
                    max_seq,
                    has_event: acc.has_event,
                }
            },
        ));

    let aggregated_by_entity = aggregated
        .map_index(|((entity, at_tick), aggregate)| (*entity, (*at_tick, aggregate.clone())));

    aggregated_by_entity.join(
        &health_indexed,
        |_entity, &(at_tick, ref aggregate), state| {
            let current = i32::from(state.current);
            let max_value = i32::from(state.max);
            let proposed = current + aggregate.net;
            let clamped = proposed.clamp(0, max_value);
            let delta = clamped - current;
            let death = aggregate.has_event && current > 0 && clamped == 0;

            HealthDelta {
                entity: state.entity,
                at_tick,
                seq: aggregate.max_seq,
                delta,
                death,
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbsp_circuit::Position;
    use dbsp::{operator::Generator, Circuit, RootCircuit};
    use ordered_float::OrderedFloat;
    use rstest::rstest;
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

    type FallDamageHarness = (
        dbsp::CircuitHandle,
        dbsp::ZSetHandle<PositionFloor>,
        dbsp::ZSetHandle<PositionFloor>,
        dbsp::ZSetHandle<Velocity>,
        dbsp::OutputHandle<OrdZSet<DamageEvent>>,
    );

    fn build_circuit() -> FallDamageHarness {
        let (circuit, (standing_in, unsupported_in, velocity_in, output)) =
            RootCircuit::build(|circuit| {
                let (standing_stream, standing_in) = circuit.add_input_zset::<PositionFloor>();
                let (unsupported_stream, unsupported_in) =
                    circuit.add_input_zset::<PositionFloor>();
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
            .map(|(event, _, weight)| {
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

        for (event, _, weight) in output.consolidate().iter() {
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
        let event = events[0];
        assert_eq!(event.entity, 1);
        assert_eq!(event.source, DamageSource::Fall);
        let expected_amount = ((8.0_f64.min(TERMINAL_VELOCITY) - SAFE_LANDING_SPEED)
            * FALL_DAMAGE_SCALE)
            .min(f64::from(u16::MAX))
            .floor() as u16;
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
            .floor() as u16;
        let expected_b = ((12.0_f64.min(TERMINAL_VELOCITY) - SAFE_LANDING_SPEED)
            * FALL_DAMAGE_SCALE)
            .min(f64::from(u16::MAX))
            .floor() as u16;

        assert_eq!(events[0].entity, 1);
        assert_eq!(events[0].source, DamageSource::Fall);
        assert_eq!(events[0].amount, expected_a);
        assert_eq!(events[0].at_tick, 1);

        assert_eq!(events[1].entity, 2);
        assert_eq!(events[1].source, DamageSource::Fall);
        assert_eq!(events[1].amount, expected_b);
        assert_eq!(events[1].at_tick, 1);
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
        assert_eq!(initial_events[0].1, 1);
        let first_event = initial_events[0].0;

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
        assert_eq!(final_events[0].1, 1);
        let final_event = final_events[0].0;

        assert!(final_event.at_tick > first_event.at_tick);
        assert_eq!(cumulative.len(), 2);
    }
}
