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

    /// Starts a fresh per-frame rollback log — the first step of the
    /// frame-rollback lifecycle that keeps the Rust-side tracking
    /// (`health_snapshot`, `pending_damage_retractions`, `applied_unsequenced`)
    /// consistent with the circuit even when a step fails.
    ///
    /// # Lifecycle
    ///
    /// Each frame the input cache pass and the output pass cooperate:
    /// 1. [`begin_frame_rollback`](Self::begin_frame_rollback) clears the
    ///    previous frame's rollback log at the top of the cache pass.
    /// 2. [`record_unsequenced_undo`](Self::record_unsequenced_undo) captures an
    ///    `applied_unsequenced` entry's pre-frame value *before* it is mutated.
    /// 3. [`stash_frame_rollback`](Self::stash_frame_rollback) saves the health
    ///    snapshot and pending-damage values the cache pass already extracted.
    /// 4. Then exactly one of, after the circuit step:
    ///    - [`commit_frame_tracking`](Self::commit_frame_tracking) on success —
    ///      discards the log; the advanced tracking stands.
    ///    - [`rollback_frame_tracking`](Self::rollback_frame_tracking) on failure
    ///      (whose inputs were cleared) — restores the exact pre-frame state.
    ///
    /// Backups reuse values the cache pass already moved out of the live state,
    /// so no frame deep-clones the whole tracking state.
    ///
    /// # Examples
    ///
    /// Successful frame keeps the advanced tracking:
    /// ```text
    /// state.begin_frame_rollback();
    /// state.record_unsequenced_undo(entity);          // capture pre-frame entry
    /// state.applied_unsequenced.insert(entity, next); // cache pass mutates it
    /// state.stash_frame_rollback(prev_snapshots, prev_pending);
    /// // ... step succeeds ...
    /// state.commit_frame_tracking();                  // applied_unsequenced == next
    /// ```
    ///
    /// Failed frame restores the pre-frame tracking:
    /// ```text
    /// state.begin_frame_rollback();
    /// state.record_unsequenced_undo(entity);
    /// state.applied_unsequenced.insert(entity, next);
    /// state.stash_frame_rollback(prev_snapshots, prev_pending);
    /// // ... step fails; inputs cleared ...
    /// state.rollback_frame_tracking();                // entry back to pre-frame value
    /// ```
    pub(crate) fn begin_frame_rollback(&mut self) {
        self.clear_frame_rollback();
    }

    /// Clears the per-frame rollback log — both backup slots and the
    /// `applied_unsequenced` undo map. Shared by [`Self::begin_frame_rollback`]
    /// (fresh frame) and [`Self::commit_frame_tracking`] (frame committed) so
    /// both paths stay identical as tracking fields evolve.
    fn clear_frame_rollback(&mut self) {
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
        self.clear_frame_rollback();
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
mod tests;
