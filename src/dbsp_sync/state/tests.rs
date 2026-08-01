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
fn new_state_starts_empty(#[from(state)] state_result: Result<DbspState, dbsp::Error>) {
    let state = state_result.expect("failed to initialise DbspState for tests");
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
fn entity_lookup_uses_mapping(#[from(state)] state_result: Result<DbspState, dbsp::Error>) {
    let mut state = state_result.expect("failed to initialise DbspState for tests");
    let entity = Entity::from_bits(42);
    state.id_map.insert(7, entity);
    state.rev_map.insert(entity, 7);
    assert_eq!(state.entity_for_id(7), Some(entity));
    assert!(state.entity_for_id(8).is_none());
}

#[rstest]
fn duplicate_counter_reports_value(#[from(state)] state_result: Result<DbspState, dbsp::Error>) {
    let mut state = state_result.expect("failed to initialise DbspState for tests");
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

/// `stash_frame_rollback` keeps the first pre-frame values it is given and
/// ignores later calls within the same frame, so a repeat call cannot latch
/// already-advanced state. Removing either `is_none` guard makes the rollback
/// below restore the second (wrong) values and fails this test.
#[rstest]
fn stash_frame_rollback_keeps_first_values(
    #[from(state)] state_result: Result<DbspState, dbsp::Error>,
) {
    let mut state = state_result.expect("failed to initialise DbspState for tests");
    let first_snapshot = HealthState {
        entity: 3,
        current: 50,
        max: 100,
    };
    let first_pending = damage_event(3, 1);
    // Distinct from the first pair, so a lost guard is observable.
    let second_snapshot = HealthState {
        entity: 3,
        current: 10,
        max: 100,
    };
    let second_pending = damage_event(3, 2);

    state.begin_frame_rollback();
    state.stash_frame_rollback(vec![first_snapshot], vec![first_pending]);
    state.stash_frame_rollback(vec![second_snapshot], vec![second_pending]);

    state.rollback_frame_tracking();

    assert_eq!(
        state.health_snapshot.get(&3),
        Some(&first_snapshot),
        "rollback must restore the health snapshot from the first stash"
    );
    assert_eq!(
        state.pending_damage_retractions,
        vec![first_pending],
        "rollback must restore the pending damage from the first stash"
    );
}

/// Bounded state-transition matrix over the rollback-relevant combinations:
/// whether the entity had a prior `applied_unsequenced` entry, whether the
/// undo was recorded once or twice (the second must be a no-op), and whether
/// the frame commits or rolls back. Rollback must restore the exact pre-frame
/// entry (removing entries that were absent before the frame); commit must
/// make a later rollback a no-op.
#[rstest]
#[case::no_prior_single_rollback(false, false, false)]
#[case::no_prior_repeat_rollback(false, true, false)]
#[case::prior_single_rollback(true, false, false)]
#[case::prior_repeat_rollback(true, true, false)]
#[case::no_prior_single_commit(false, false, true)]
#[case::no_prior_repeat_commit(false, true, true)]
#[case::prior_single_commit(true, false, true)]
#[case::prior_repeat_commit(true, true, true)]
fn applied_unsequenced_rollback_matrix(
    #[case] had_prior: bool,
    #[case] repeat_undo: bool,
    #[case] commit: bool,
    #[from(state)] state_result: Result<DbspState, dbsp::Error>,
) {
    let mut state = state_result.expect("failed to initialise DbspState for tests");
    let entity: EntityId = 7;
    let prior = (1, HashSet::from([damage_event(entity, 1)]));
    if had_prior {
        state.applied_unsequenced.insert(entity, prior.clone());
    }

    state.begin_frame_rollback();
    state.record_unsequenced_undo(entity);
    let advanced = (2, HashSet::from([damage_event(entity, 2)]));
    state.applied_unsequenced.insert(entity, advanced.clone());
    if repeat_undo {
        // Recorded *after* the mutation: a repeat capture for the same entity
        // must keep the original pre-frame value rather than latching the
        // already-advanced entry.
        state.record_unsequenced_undo(entity);
    }
    state.stash_frame_rollback(Vec::new(), Vec::new());

    if commit {
        state.commit_frame_tracking();
        // A stray rollback after commit must not revert the committed state.
        state.rollback_frame_tracking();
        assert_eq!(
            state.applied_unsequenced.get(&entity),
            Some(&advanced),
            "committed state must survive a later rollback \
             (had_prior={had_prior}, repeat_undo={repeat_undo})"
        );
    } else {
        state.rollback_frame_tracking();
        let expected = had_prior.then_some(prior);
        assert_eq!(
            state.applied_unsequenced.get(&entity),
            expected.as_ref(),
            "rollback must restore the exact pre-frame applied_unsequenced \
             (had_prior={had_prior}, repeat_undo={repeat_undo})"
        );
    }
}

mod properties {
    //! Property-based coverage supplementing the exhaustive matrix above.
    //!
    //! [`applied_unsequenced_rollback_matrix`] enumerates the finite
    //! present/absent × mutate/leave space exactly; this samples *sequences* of
    //! lifecycle calls instead, where the interesting failures are ordering ones —
    //! a repeated stash overwriting the true pre-frame values, or a commit failing
    //! to disarm a later rollback. See
    //! `docs/adr-003-bounded-rstest-over-property-testing.md`.

    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    /// One call in the frame-rollback protocol.
    #[derive(Clone, Copy, Debug)]
    enum Action {
        /// `begin_frame_rollback` — a new cache pass starts.
        Begin,
        /// A bare `record_unsequenced_undo` with no following mutation.
        /// Repeats must be no-ops.
        RecordUndo(u8),
        /// The production pattern: record the undo, then mutate the entry.
        ApplyUnsequenced(u8, u8),
        /// Advance the health/pending tracking and stash the pre-values, as the
        /// cache pass does. Repeats within a frame must not overwrite the
        /// stashed values with already-advanced ones.
        Stash(u8),
        /// `commit_frame_tracking` — the step succeeded.
        Commit,
        /// `rollback_frame_tracking` — the step failed.
        Rollback,
    }

    fn action_strategy() -> impl Strategy<Value = Action> {
        prop_oneof![
            2 => Just(Action::Begin),
            2 => (0u8..3).prop_map(Action::RecordUndo),
            4 => (0u8..3, 0u8..4).prop_map(|(e, v)| Action::ApplyUnsequenced(e, v)),
            4 => (0u8..4).prop_map(Action::Stash),
            2 => Just(Action::Commit),
            2 => Just(Action::Rollback),
        ]
    }

    /// The tracking fields the frame-rollback protocol governs, copied out
    /// eagerly for comparison. The production state keeps these lazily, which is
    /// exactly what the property is checking.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Tracking {
        applied_unsequenced: HashMap<EntityId, (Tick, HashSet<DamageEvent>)>,
        health_snapshot: HashMap<EntityId, HealthState>,
        pending_damage_retractions: Vec<DamageEvent>,
    }

    impl Tracking {
        fn of(state: &DbspState) -> Self {
            Self {
                applied_unsequenced: state.applied_unsequenced.clone(),
                health_snapshot: state.health_snapshot.clone(),
                pending_damage_retractions: state.pending_damage_retractions.clone(),
            }
        }
    }

    fn unsequenced_entry(value: u8) -> (Tick, HashSet<DamageEvent>) {
        let mut events = HashSet::new();
        events.insert(damage_event(EntityId::from(value), Tick::from(value)));
        (Tick::from(value), events)
    }

    /// Applies one action to the state, driving it exactly as the cache and
    /// output passes do.
    fn apply_action(state: &mut DbspState, action: Action) {
        match action {
            Action::Begin => state.begin_frame_rollback(),
            Action::RecordUndo(entity) => state.record_unsequenced_undo(EntityId::from(entity)),
            Action::ApplyUnsequenced(entity, value) => {
                let entity_id = EntityId::from(entity);
                state.record_unsequenced_undo(entity_id);
                state
                    .applied_unsequenced
                    .insert(entity_id, unsequenced_entry(value));
            }
            Action::Stash(value) => {
                // The cache pass extracts the live values, then advances them.
                let previous_snapshots: Vec<_> = state.health_snapshot.values().copied().collect();
                let previous_pending = std::mem::take(&mut state.pending_damage_retractions);
                state.health_snapshot.insert(
                    EntityId::from(value),
                    HealthState {
                        entity: EntityId::from(value),
                        current: u16::from(value),
                        max: 100,
                    },
                );
                state
                    .pending_damage_retractions
                    .push(damage_event(EntityId::from(value), Tick::from(value)));
                state.stash_frame_rollback(previous_snapshots, previous_pending);
            }
            Action::Commit => state.commit_frame_tracking(),
            Action::Rollback => state.rollback_frame_tracking(),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Reference model: as long as every mutation follows the protocol
        /// (record the undo, stash before advancing), the tracking at any
        /// rollback equals the tracking as it stood at the last `Begin` or
        /// `Commit`, and a commit leaves the advanced tracking in place.
        #[test]
        fn rollback_restores_the_frame_start_tracking(
            actions in prop::collection::vec(action_strategy(), 1..24),
        ) {
            let mut state = DbspState::new()
                .map_err(|error| TestCaseError::fail(format!("DbspState::new failed: {error}")))?;
            // The model: what the tracking looked like when this frame began.
            let mut frame_start = Tracking::of(&state);

            for action in actions {
                let before = Tracking::of(&state);
                apply_action(&mut state, action);

                match action {
                    // A new frame re-bases the model on the current tracking.
                    Action::Begin => frame_start = Tracking::of(&state),
                    // Commit must preserve the advanced tracking, and re-base
                    // the model so a later rollback restores nothing.
                    Action::Commit => {
                        prop_assert_eq!(
                            Tracking::of(&state),
                            before,
                            "commit must leave the advanced tracking untouched"
                        );
                        frame_start = Tracking::of(&state);
                    }
                    // Rollback must restore the exact frame-start tracking.
                    Action::Rollback => {
                        prop_assert_eq!(
                            Tracking::of(&state),
                            frame_start.clone(),
                            "rollback must restore the exact frame-start tracking"
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}
