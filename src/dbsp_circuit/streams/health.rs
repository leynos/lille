//! Health aggregation streams.
//!
//! These helpers reduce health snapshots and incoming damage events to
//! authoritative [`HealthDelta`] records emitted by the DBSP circuit.

use std::cmp::max;

use dbsp::{algebra::Semigroup, operator::Fold, typed_batch::OrdZSet, RootCircuit, Stream};

use crate::dbsp_circuit::{DamageEvent, DamageSource, HealthDelta, HealthState};

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
struct HealthAccumulator {
    entries: Vec<(Option<u32>, i32)>,
    has_event: bool,
}

impl HealthAccumulator {
    fn insert(&mut self, seq: Option<u32>, signed: i32) {
        match self
            .entries
            .binary_search_by(|(existing, _)| existing.cmp(&seq))
        {
            Ok(_) => {}
            Err(pos) => {
                self.entries.insert(pos, (seq, signed));
                self.has_event = true;
            }
        }
    }

    fn merge(&mut self, other: &Self) {
        for (seq, signed) in &other.entries {
            match self
                .entries
                .binary_search_by(|(existing, _)| existing.cmp(seq))
            {
                Ok(_) => {}
                Err(pos) => {
                    self.entries.insert(pos, (*seq, *signed));
                }
            }
        }
        self.has_event |= other.has_event;
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
                if weight <= 0 {
                    return;
                }
                let signed = signed_amount(event);
                acc.insert(event.seq, signed);
            },
            |acc: HealthAccumulator| {
                let mut net = 0;
                let mut max_seq = None;
                for (seq, signed) in &acc.entries {
                    net += *signed;
                    if let Some(value) = seq {
                        max_seq = Some(match max_seq {
                            Some(existing) => max(existing, *value),
                            None => *value,
                        });
                    }
                }
                HealthAggregate {
                    net,
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
