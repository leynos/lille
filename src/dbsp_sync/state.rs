//! State tracking for DBSP synchronisation.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Added, Changed, Entity, Query, RemovedComponents};
use log::debug;

use crate::components::DdlogId;
use crate::dbsp_circuit::{DamageEvent, DbspCircuit, EntityId, HealthState, Tick};

/// Resource storing the DBSP circuit and deduplication state.
pub struct DbspState {
    pub(crate) circuit: DbspCircuit,
    /// Cached mapping from DBSP entity IDs to Bevy `Entity` values.
    ///
    /// The map is maintained incrementally by
    /// [`cache_state_for_dbsp_system`] to avoid rebuilding it every frame.
    pub(crate) id_map: HashMap<i64, Entity>,
    /// Reverse mapping from Bevy [`Entity`] values to DBSP identifiers.
    pub(crate) rev_map: HashMap<Entity, i64>,
    pub(crate) applied_health: HashMap<EntityId, (Tick, Option<u32>)>,
    /// Tracks unsequenced damage events applied per entity per tick.
    /// Used to detect and filter duplicate unsequenced events within the same tick.
    pub(crate) applied_unsequenced: HashMap<EntityId, (Tick, HashSet<DamageEvent>)>,
    /// Caches the last health state pushed to the circuit for each entity.
    /// Used to generate retractions when health state changes.
    pub(crate) health_snapshot: HashMap<EntityId, HealthState>,
    /// Tracks damage events that were retracted in the current frame.
    /// Used to filter out corresponding health deltas to avoid double-application.
    pub(crate) expected_health_retractions: HashSet<(EntityId, Tick, Option<u32>)>,
    /// Damage events pending retraction at the start of the next frame.
    pub(crate) pending_damage_retractions: Vec<DamageEvent>,
    /// Running count of duplicate health/damage events filtered.
    /// Used for diagnostics and monitoring deduplication effectiveness.
    pub(crate) health_duplicate_count: u64,
}

#[derive(SystemParam)]
pub struct IdQueries<'w, 's> {
    pub added: Query<'w, 's, (Entity, &'static DdlogId), Added<DdlogId>>,
    pub changed: Query<'w, 's, (Entity, &'static DdlogId), Changed<DdlogId>>,
    pub removed: RemovedComponents<'w, 's, DdlogId>,
}

impl DbspState {
    /// Creates a new [`DbspState`] with an initialised circuit.
    pub fn new() -> Result<Self, dbsp::Error> {
        Ok(Self {
            circuit: DbspCircuit::new()?,
            id_map: HashMap::new(),
            rev_map: HashMap::new(),
            applied_health: HashMap::new(),
            applied_unsequenced: HashMap::new(),
            health_snapshot: HashMap::new(),
            expected_health_retractions: HashSet::new(),
            pending_damage_retractions: Vec::new(),
            health_duplicate_count: 0,
        })
    }

    /// Looks up the Bevy [`Entity`] for a DBSP identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lille::dbsp_sync::DbspState;
    /// let state = DbspState::new().expect("failed to initialise DbspState");
    /// assert!(state.entity_for_id(42).is_none());
    /// ```
    pub fn entity_for_id(&self, id: i64) -> Option<Entity> {
        self.id_map.get(&id).copied()
    }

    /// Returns the number of duplicate health or damage events filtered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lille::dbsp_sync::DbspState;
    /// let state = DbspState::new().expect("failed to initialise DbspState");
    /// assert_eq!(state.applied_health_duplicates(), 0);
    /// ```
    pub fn applied_health_duplicates(&self) -> u64 {
        self.health_duplicate_count
    }

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
