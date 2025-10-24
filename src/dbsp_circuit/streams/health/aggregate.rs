//! Health delta aggregation streams.
//!
//! Collapses incoming damage events and health snapshots into consolidated
//! [`HealthDelta`] records for each entity.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::dbsp_circuit::{DamageEvent, DamageSource, HealthDelta, HealthState};
use dbsp::{algebra::Semigroup, operator::Fold, typed_batch::OrdZSet, RootCircuit, Stream};

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
                        existing_signed, signed,
                        "sequenced damage event mismatch for seq {seq}: \
                         existing {existing_signed}, incoming {signed}"
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
                        existing_signed, incoming_signed,
                        "sequenced damage event mismatch for seq {seq_value}: \
                         existing {existing_signed}, incoming {incoming_signed}"
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
        DamageSource::External | DamageSource::Fall | DamageSource::Other { .. } => {
            -i32::from(event.amount)
        }
    }
}

/// Aggregates health state and damage inputs into canonical [`HealthDelta`]
/// records.
///
/// # Examples
/// ```rust,no_run
/// # use anyhow::Error;
/// # use dbsp::RootCircuit;
/// # use lille::dbsp_circuit::{
/// #     streams::health::aggregate::health_delta_stream, DamageEvent, HealthState,
/// # };
/// # let _ = RootCircuit::build(|circuit| -> Result<(), Error> {
/// #     let (states, _) = circuit.add_input_zset::<HealthState>();
/// #     let (events, _) = circuit.add_input_zset::<DamageEvent>();
/// #     let _ = health_delta_stream(&states, &events);
/// #     Ok(())
/// # });
/// ```
#[must_use]
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
