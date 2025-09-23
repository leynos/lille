//! Duplicate filtering helpers for DBSP damage ingestion.

use std::collections::HashSet;

use log::debug;

use crate::dbsp_circuit::{DamageEvent, EntityId, Tick};

use super::DbspState;

impl DbspState {
    /// Sequenced damage ingestion uses first-write-wins, so later events with
    /// the same `(entity, tick, seq)` are ignored and the circuit only processes
    /// the earliest payload.
    pub(crate) fn record_duplicate_sequenced_damage(
        &mut self,
        event: &DamageEvent,
        seen: &mut HashSet<(EntityId, Tick, u32)>,
    ) -> bool {
        let Some(seq) = event.seq else {
            return false;
        };

        if self.applied_health.get(&event.entity) == Some(&(event.at_tick, Some(seq))) {
            debug!(
                "duplicate damage event ignored for entity {} at tick {} seq {}",
                event.entity, event.at_tick, seq
            );
            self.health_duplicate_count += 1;
            return true;
        }

        let key = (event.entity, event.at_tick, seq);
        if !seen.insert(key) {
            debug!(
                "duplicate damage event ignored for entity {} at tick {} seq {}",
                event.entity, event.at_tick, seq
            );
            self.health_duplicate_count += 1;
            return true;
        }

        false
    }

    pub(crate) fn record_duplicate_unsequenced_damage(
        &mut self,
        event: &DamageEvent,
        seen: &mut HashSet<DamageEvent>,
    ) -> bool {
        let entry = self
            .applied_unsequenced
            .entry(event.entity)
            .or_insert_with(|| (event.at_tick, HashSet::new()));
        if entry.0 != event.at_tick {
            entry.0 = event.at_tick;
            entry.1.clear();
        }
        if entry.1.contains(event) || !seen.insert(*event) {
            debug!(
                "duplicate unsequenced damage event ignored for entity {} at tick {}",
                event.entity, event.at_tick
            );
            self.health_duplicate_count += 1;
            return true;
        }
        entry.1.insert(*event);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbsp_circuit::DamageSource;
    use rstest::rstest;
    use std::collections::HashSet;

    fn fresh_state() -> DbspState {
        DbspState::new().expect("failed to initialise DbspState for tests")
    }

    fn sequenced_event(seq: u32) -> DamageEvent {
        DamageEvent {
            entity: 1,
            amount: 10,
            source: DamageSource::External,
            at_tick: 1,
            seq: Some(seq),
        }
    }

    fn unsequenced_event(at_tick: u64, amount: u16) -> DamageEvent {
        DamageEvent {
            entity: 1,
            amount,
            source: DamageSource::External,
            at_tick,
            seq: None,
        }
    }

    #[rstest]
    fn sequenced_duplicate_is_counted() {
        let mut state = fresh_state();
        let mut seen = HashSet::new();
        let event = sequenced_event(5);
        assert!(!state.record_duplicate_sequenced_damage(&event, &mut seen));
        assert_eq!(state.applied_health_duplicates(), 0);
        assert!(state.record_duplicate_sequenced_damage(&event, &mut seen));
        assert_eq!(state.applied_health_duplicates(), 1);
    }

    #[rstest]
    fn sequenced_event_reapplied_in_next_frame_is_ignored() {
        let mut state = fresh_state();
        let mut seen = HashSet::new();
        let event = sequenced_event(7);
        state
            .applied_health
            .insert(event.entity, (event.at_tick, event.seq));
        assert!(state.record_duplicate_sequenced_damage(&event, &mut seen));
        assert_eq!(state.applied_health_duplicates(), 1);
    }

    #[rstest]
    fn unsequenced_duplicate_is_counted() {
        let mut state = fresh_state();
        let mut seen = HashSet::new();
        let event = unsequenced_event(2, 12);
        assert!(!state.record_duplicate_unsequenced_damage(&event, &mut seen));
        assert!(state.record_duplicate_unsequenced_damage(&event, &mut seen));
        assert_eq!(state.applied_health_duplicates(), 1);
    }

    #[rstest]
    fn unsequenced_events_reset_each_tick() {
        let mut state = fresh_state();
        let mut first_seen = HashSet::new();
        let mut second_seen = HashSet::new();
        let first = unsequenced_event(3, 5);
        let second = unsequenced_event(4, 5);
        assert!(!state.record_duplicate_unsequenced_damage(&first, &mut first_seen));
        assert!(!state.record_duplicate_unsequenced_damage(&second, &mut second_seen));
        assert_eq!(state.applied_health_duplicates(), 0);
    }
}
