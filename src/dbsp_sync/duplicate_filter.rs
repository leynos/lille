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
