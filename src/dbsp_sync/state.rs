//! State tracking for DBSP synchronisation.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Added, Changed, Entity, Query, RemovedComponents};

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
/// Convenience wrapper exposing queries required to track `DdlogId` changes.
pub struct IdQueries<'w, 's> {
    /// Entities that gained a `DdlogId` this frame.
    pub added: Query<'w, 's, (Entity, &'static DdlogId), Added<DdlogId>>,
    /// Entities whose `DdlogId` component changed.
    pub changed: Query<'w, 's, (Entity, &'static DdlogId), Changed<DdlogId>>,
    /// Entities that lost their `DdlogId` component.
    pub removed: RemovedComponents<'w, 's, DdlogId>,
}

impl DbspState {
    /// Creates a new [`DbspState`] with an initialised circuit.
    ///
    /// # Errors
    /// Returns a DBSP error if the underlying circuit fails to construct.
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
    #[must_use]
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
    #[must_use]
    pub const fn applied_health_duplicates(&self) -> u64 {
        self.health_duplicate_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Entity;
    use rstest::rstest;

    #[rstest]
    fn new_state_starts_empty() {
        let state = DbspState::new().expect("failed to initialise DbspState for tests");
        assert!(state.id_map.is_empty());
        assert!(state.rev_map.is_empty());
        assert!(state.applied_health.is_empty());
        assert!(state.applied_unsequenced.is_empty());
        assert!(state.health_snapshot.is_empty());
        assert!(state.expected_health_retractions.is_empty());
        assert!(state.pending_damage_retractions.is_empty());
        assert_eq!(state.applied_health_duplicates(), 0);
    }

    #[rstest]
    fn entity_lookup_uses_mapping() {
        let mut state = DbspState::new().expect("failed to initialise DbspState for tests");
        let entity = Entity::from_raw(42);
        state.id_map.insert(7, entity);
        state.rev_map.insert(entity, 7);
        assert_eq!(state.entity_for_id(7), Some(entity));
        assert!(state.entity_for_id(8).is_none());
    }

    #[rstest]
    fn duplicate_counter_reports_value() {
        let mut state = DbspState::new().expect("failed to initialise DbspState for tests");
        state.health_duplicate_count = 3;
        assert_eq!(state.applied_health_duplicates(), 3);
    }
}
