//! State tracking for DBSP synchronisation.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Added, Changed, Entity, Query, RemovedComponents};

use crate::components::DdlogId;
use crate::dbsp_circuit::{try_step, DamageEvent, DbspCircuit, EntityId, HealthState, Tick};

/// Resource storing the DBSP circuit and deduplication state.
pub struct DbspState {
    pub(crate) circuit: DbspCircuit,
    /// Function pointer used to advance the circuit; overridden in tests to
    /// force error paths without mutating the real DBSP logic.
    stepper: fn(&mut DbspCircuit) -> Result<(), dbsp::Error>,
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
    /// Pre-frame health snapshots the cache pass drains out of
    /// [`Self::health_snapshot`], stashed (not cloned) so a failed circuit step
    /// can rebuild the map from them.
    health_snapshot_backup: Option<Vec<HealthState>>,
    /// Pre-frame value of [`Self::pending_damage_retractions`] the cache pass
    /// takes, restored on a failed circuit step.
    pending_damage_backup: Option<Vec<DamageEvent>>,
    /// Undo log of [`Self::applied_unsequenced`] entries mutated during the
    /// cache pass. Records each touched entity's prior value once, so a failed
    /// step can restore it without deep-cloning the whole map every frame.
    applied_unsequenced_undo: HashMap<EntityId, Option<(Tick, HashSet<DamageEvent>)>>,
    /// Running count of duplicate health/damage events filtered.
    /// Used for diagnostics and monitoring deduplication effectiveness.
    pub(crate) health_duplicate_count: u64,
}

/// Convenience wrapper exposing queries required to track `DdlogId` changes.
#[derive(SystemParam)]
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
    #[must_use = "DbspState initialisation may fail; handle the Result"]
    pub fn new() -> Result<Self, dbsp::Error> {
        Ok(Self {
            circuit: DbspCircuit::new()?,
            stepper: try_step,
            id_map: HashMap::new(),
            rev_map: HashMap::new(),
            applied_health: HashMap::new(),
            applied_unsequenced: HashMap::new(),
            health_snapshot: HashMap::new(),
            expected_health_retractions: HashSet::new(),
            pending_damage_retractions: Vec::new(),
            health_snapshot_backup: None,
            pending_damage_backup: None,
            applied_unsequenced_undo: HashMap::new(),
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

    /// Invokes the configured circuit stepper.
    pub(crate) fn step_circuit(&mut self) -> Result<(), dbsp::Error> {
        (self.stepper)(&mut self.circuit)
    }

    /// Starts a fresh per-frame rollback log at the top of a cache pass.
    ///
    /// The health/damage backups are populated later ([`Self::stash_frame_rollback`])
    /// from values the cache pass has already moved out of the live state, and
    /// the `applied_unsequenced` undo log is populated lazily
    /// ([`Self::record_unsequenced_undo`]) as entries are mutated. This avoids
    /// deep-cloning the whole tracking state every frame.
    pub(crate) fn begin_frame_rollback(&mut self) {
        self.health_snapshot_backup = None;
        self.pending_damage_backup = None;
        self.applied_unsequenced_undo.clear();
    }

    /// Stashes the pre-frame health snapshots and pending damage retractions —
    /// values the cache pass has already extracted from the live state — so a
    /// failed step can restore them without an extra clone.
    pub(crate) fn stash_frame_rollback(
        &mut self,
        health_snapshot: Vec<HealthState>,
        pending_damage: Vec<DamageEvent>,
    ) {
        self.health_snapshot_backup = Some(health_snapshot);
        self.pending_damage_backup = Some(pending_damage);
    }

    /// Records the pre-frame [`Self::applied_unsequenced`] entry for `entity`
    /// once per frame, before the cache pass mutates it, so a failed step can
    /// undo the change. Repeat calls for the same entity in a frame are no-ops.
    pub(crate) fn record_unsequenced_undo(&mut self, entity: EntityId) {
        if !self.applied_unsequenced_undo.contains_key(&entity) {
            let previous = self.applied_unsequenced.get(&entity).cloned();
            self.applied_unsequenced_undo.insert(entity, previous);
        }
    }

    /// Discards the frame rollback log once a successful step has committed the
    /// frame's circuit inputs.
    pub(crate) fn commit_frame_tracking(&mut self) {
        self.health_snapshot_backup = None;
        self.pending_damage_backup = None;
        self.applied_unsequenced_undo.clear();
    }

    /// Restores the pre-frame health/damage tracking after a failed step whose
    /// circuit inputs were cleared without being applied, keeping the Rust-side
    /// bookkeeping consistent with the circuit's actual records. A no-op when no
    /// backup was taken (e.g. the output system run in isolation by a test).
    pub(crate) fn rollback_frame_tracking(&mut self) {
        if let Some(snapshots) = self.health_snapshot_backup.take() {
            self.health_snapshot = snapshots
                .into_iter()
                .map(|snapshot| (snapshot.entity, snapshot))
                .collect();
        }
        if let Some(pending) = self.pending_damage_backup.take() {
            self.pending_damage_retractions = pending;
        }
        for (entity, previous) in std::mem::take(&mut self.applied_unsequenced_undo) {
            match previous {
                Some(entry) => {
                    self.applied_unsequenced.insert(entity, entry);
                }
                None => {
                    self.applied_unsequenced.remove(&entity);
                }
            }
        }
    }

    /// Overrides the circuit stepper for tests that need to force an error
    /// path without mutating the DBSP logic.
    ///
    /// Only compiled for unit tests or when the `test-support` feature is
    /// enabled so production code cannot swap the stepper accidentally.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn set_stepper_for_testing(
        &mut self,
        stepper: fn(&mut DbspCircuit) -> Result<(), dbsp::Error>,
    ) {
        self.stepper = stepper;
    }
}

#[cfg(test)]
mod tests {
    //! Tests for the DBSP synchronisation state resource.
    use super::*;
    use bevy::prelude::Entity;
    use rstest::{fixture, rstest};

    /// Shared fresh [`DbspState`] for the frame-rollback tests below.
    ///
    /// Returns the fallible `Result` so construction stays outside a
    /// `no_expect_outside_tests` boundary; each test unwraps it, mirroring the
    /// `setup_app`/`fresh_state` helpers used elsewhere.
    #[fixture]
    fn state() -> Result<DbspState, dbsp::Error> {
        DbspState::new()
    }

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
        let entity = Entity::from_bits(42);
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

    fn damage_event(entity: EntityId, at_tick: Tick) -> DamageEvent {
        DamageEvent {
            entity,
            amount: 10,
            source: crate::dbsp_circuit::DamageSource::External,
            at_tick,
            seq: None,
        }
    }

    #[rstest]
    fn rollback_restores_health_snapshot_and_pending_damage(
        #[from(state)] state_result: Result<DbspState, dbsp::Error>,
    ) {
        let mut state = state_result.expect("failed to initialise DbspState for tests");
        let snapshot = HealthState {
            entity: 3,
            current: 50,
            max: 100,
        };
        let pending = damage_event(3, 1);
        state.health_snapshot.insert(3, snapshot);
        state.pending_damage_retractions.push(pending);

        // Simulate a cache pass: back up, drain/advance the live tracking.
        state.begin_frame_rollback();
        let previous_snapshots: Vec<_> = state.health_snapshot.values().copied().collect();
        state.health_snapshot.clear();
        let previous_pending = std::mem::take(&mut state.pending_damage_retractions);
        state.health_snapshot.insert(
            3,
            HealthState {
                entity: 3,
                current: 10,
                max: 100,
            },
        );
        state.pending_damage_retractions.push(damage_event(3, 2));
        state.stash_frame_rollback(previous_snapshots, previous_pending);

        state.rollback_frame_tracking();

        assert_eq!(state.health_snapshot.get(&3), Some(&snapshot));
        assert_eq!(state.pending_damage_retractions, vec![pending]);
    }

    #[rstest]
    fn rollback_restores_applied_unsequenced(
        #[from(state)] state_result: Result<DbspState, dbsp::Error>,
    ) {
        let mut state = state_result.expect("failed to initialise DbspState for tests");
        let original = damage_event(7, 1);
        state
            .applied_unsequenced
            .insert(7, (1, HashSet::from([original])));

        state.begin_frame_rollback();
        // Entity 7 already had an entry; entity 8 is new this frame.
        state.record_unsequenced_undo(7);
        state.applied_unsequenced.insert(7, (2, HashSet::new()));
        state.record_unsequenced_undo(8);
        state.applied_unsequenced.insert(8, (2, HashSet::new()));
        // A repeat undo record for 7 must not overwrite the captured value.
        state.record_unsequenced_undo(7);
        state.stash_frame_rollback(Vec::new(), Vec::new());

        state.rollback_frame_tracking();

        assert_eq!(
            state.applied_unsequenced.get(&7),
            Some(&(1, HashSet::from([original]))),
            "entity 7 restored to its pre-frame bucket"
        );
        assert!(
            !state.applied_unsequenced.contains_key(&8),
            "entity 8 was absent pre-frame, so it is removed on rollback"
        );
    }

    #[rstest]
    fn commit_discards_rollback_log(#[from(state)] state_result: Result<DbspState, dbsp::Error>) {
        let mut state = state_result.expect("failed to initialise DbspState for tests");
        state.begin_frame_rollback();
        state.record_unsequenced_undo(5);
        state.applied_unsequenced.insert(5, (9, HashSet::new()));
        state.stash_frame_rollback(Vec::new(), Vec::new());

        state.commit_frame_tracking();
        // A stray rollback after commit must not revert the committed state.
        state.rollback_frame_tracking();

        assert_eq!(
            state.applied_unsequenced.get(&5).map(|(tick, _)| *tick),
            Some(9),
            "committed applied_unsequenced state must survive a later rollback"
        );
    }
}
