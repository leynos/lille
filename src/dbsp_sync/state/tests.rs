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
    if repeat_undo {
        // A repeat record for the same entity must not overwrite the capture.
        state.record_unsequenced_undo(entity);
    }
    let advanced = (2, HashSet::from([damage_event(entity, 2)]));
    state.applied_unsequenced.insert(entity, advanced.clone());
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
