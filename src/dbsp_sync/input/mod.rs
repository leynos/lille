//! Input synchronisation systems bridging Bevy ECS into the DBSP circuit.

use std::{collections::HashSet, mem};

use bevy::prelude::*;

use crate::components::{
    Block, BlockSlope, DdlogId, ForceComp, Health, Target as TargetComp, VelocityComp,
};
use crate::dbsp_circuit::{DamageEvent, DbspCircuit, HealthState};
#[cfg(feature = "map")]
use crate::map::{PlayerSpawn, SpawnPoint};
use crate::world_handle::WorldHandle;

use super::{DamageInbox, DbspState, IdQueries};

type EntityRow<'w> = (
    Entity,
    &'w DdlogId,
    &'w Transform,
    Option<&'w VelocityComp>,
    Option<&'w TargetComp>,
    Option<&'w mut Health>,
);

mod sync;

/// Initializes the [`DbspState`] resource in the provided [`World`].
///
/// Call this once during Bevy startup before running any DBSP synchronisation
/// systems.
///
/// # Errors
/// Returns any error produced while constructing the DBSP circuit.
pub fn init_dbsp_system(world: &mut World) -> Result<(), dbsp::Error> {
    let state = DbspState::new()?;
    world.insert_non_send_resource(state);
    Ok(())
}

/// Caches current ECS state into the DBSP circuit inputs.
///
/// This system gathers `Transform`, optional `Velocity`, `Block`, and optional
/// `Force` components and pushes them into the circuit's input handles. Forces
/// for entities not present in the current position pass are ignored. It also
/// updates the internal mapping from DBSP entity identifiers to Bevy entities,
/// ensuring the lookup is maintained without rebuilding the map each frame. It
/// also refreshes the [`WorldHandle`] resource with the same cached data for
/// tests and diagnostics.
#[cfg(not(feature = "map"))]
#[expect(
    clippy::too_many_arguments,
    reason = "System boundary requires multiple Bevy resources."
)]
pub fn cache_state_for_dbsp_system(
    mut state: NonSendMut<DbspState>,
    mut entity_query: Query<EntityRow<'_>>,
    force_query: Query<(Entity, &DdlogId, &ForceComp)>,
    block_query: Query<(&Block, Option<&BlockSlope>)>,
    mut id_queries: IdQueries,
    mut damage_inbox: ResMut<DamageInbox>,
    mut world_handle: ResMut<WorldHandle>,
) {
    cache_state_for_dbsp_impl(
        &mut state,
        &mut entity_query,
        &force_query,
        &block_query,
        &mut id_queries,
        &mut damage_inbox,
        &mut world_handle,
    );
}

/// Caches current ECS state into the DBSP circuit inputs (with map feature).
///
/// This variant includes spawn point synchronisation when the `map` feature is
/// enabled. It gathers `Transform`, optional `Velocity`, `Block`, `PlayerSpawn`,
/// and `SpawnPoint` components and pushes them into the circuit's input handles.
#[cfg(feature = "map")]
#[expect(
    clippy::too_many_arguments,
    reason = "System boundary requires multiple Bevy resources."
)]
pub fn cache_state_for_dbsp_system(
    mut state: NonSendMut<DbspState>,
    mut entity_query: Query<EntityRow<'_>>,
    force_query: Query<(Entity, &DdlogId, &ForceComp)>,
    block_query: Query<(&Block, Option<&BlockSlope>)>,
    player_spawn_query: Query<(Entity, &Transform), With<PlayerSpawn>>,
    spawn_point_query: Query<(Entity, &Transform, &SpawnPoint)>,
    mut id_queries: IdQueries,
    mut damage_inbox: ResMut<DamageInbox>,
    mut world_handle: ResMut<WorldHandle>,
) {
    sync::player_spawns(&mut state.circuit, &player_spawn_query);
    sync::spawn_points(&mut state.circuit, &spawn_point_query);
    cache_state_for_dbsp_impl(
        &mut state,
        &mut entity_query,
        &force_query,
        &block_query,
        &mut id_queries,
        &mut damage_inbox,
        &mut world_handle,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "Helper coordinates multiple Bevy resources from system boundary."
)]
fn cache_state_for_dbsp_impl(
    state: &mut DbspState,
    entity_query: &mut Query<EntityRow<'_>>,
    force_query: &Query<(Entity, &DdlogId, &ForceComp)>,
    block_query: &Query<(&Block, Option<&BlockSlope>)>,
    id_queries: &mut IdQueries,
    damage_inbox: &mut DamageInbox,
    world_handle: &mut WorldHandle,
) {
    world_handle.blocks.clear();
    world_handle.slopes.clear();
    world_handle.entities.clear();

    let previous_snapshots = collect_previous_health_snapshots(state);
    let pending_damage = mem::take(&mut state.pending_damage_retractions);
    state.expected_health_retractions.clear();

    sync::blocks(&mut state.circuit, block_query, world_handle);
    sync::id_maps(state, id_queries);
    sync::entities(state, entity_query, world_handle);
    sync::forces(state, force_query);

    apply_health_snapshot_retractions(&mut state.circuit, &previous_snapshots);
    apply_damage_retractions(state, pending_damage);

    ingest_damage_events(state, damage_inbox);
}

fn collect_previous_health_snapshots(state: &mut DbspState) -> Vec<HealthState> {
    let snapshots: Vec<_> = state.health_snapshot.values().copied().collect();
    state.health_snapshot.clear();
    snapshots
}

fn apply_health_snapshot_retractions(circuit: &mut DbspCircuit, snapshots: &[HealthState]) {
    for snapshot in snapshots {
        circuit.health_state_in().push(*snapshot, -1);
    }
}

fn apply_damage_retractions(state: &mut DbspState, retractions: Vec<DamageEvent>) {
    for event in retractions {
        state.circuit.damage_in().push(event, -1);
        state
            .expected_health_retractions
            .insert((event.entity, event.at_tick, event.seq));
    }
}

fn ingest_damage_events(state: &mut DbspState, inbox: &mut DamageInbox) {
    let mut sequenced_damage = HashSet::new();
    let mut unsequenced_damage = HashSet::new();
    for event in inbox.drain() {
        let duplicate = match event.seq {
            Some(_) => state.record_duplicate_sequenced_damage(&event, &mut sequenced_damage),
            None => state.record_duplicate_unsequenced_damage(&event, &mut unsequenced_damage),
        };
        if duplicate {
            continue;
        }
        state.circuit.damage_in().push(event, 1);
        state.pending_damage_retractions.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbsp_circuit::DamageSource;
    use rstest::rstest;

    fn make_state() -> DbspState {
        DbspState::new().expect("failed to initialise DbspState for tests")
    }

    fn sequenced_event(seq: u32) -> DamageEvent {
        DamageEvent {
            entity: 1,
            amount: 20,
            source: DamageSource::External,
            at_tick: 1,
            seq: Some(seq),
        }
    }

    fn unsequenced_event(amount: u16) -> DamageEvent {
        DamageEvent {
            entity: 1,
            amount,
            source: DamageSource::External,
            at_tick: 1,
            seq: None,
        }
    }

    #[rstest]
    fn sequenced_duplicates_are_dropped() {
        let mut state = make_state();
        let mut inbox = DamageInbox::default();
        let event = sequenced_event(3);
        inbox.extend(vec![event, event]);
        ingest_damage_events(&mut state, &mut inbox);
        assert!(inbox.is_empty());
        assert_eq!(state.pending_damage_retractions, vec![event]);
        assert_eq!(state.applied_health_duplicates(), 1);
    }

    #[rstest]
    fn unsequenced_duplicates_are_dropped() {
        let mut state = make_state();
        let mut inbox = DamageInbox::default();
        let event = unsequenced_event(15);
        inbox.extend(vec![event, event]);
        ingest_damage_events(&mut state, &mut inbox);
        assert!(inbox.is_empty());
        assert_eq!(state.pending_damage_retractions, vec![event]);
        assert_eq!(state.applied_health_duplicates(), 1);
    }

    #[rstest]
    fn unique_events_are_ingested() {
        let mut state = make_state();
        let mut inbox = DamageInbox::default();
        let sequenced = sequenced_event(4);
        let unsequenced = unsequenced_event(12);
        inbox.extend(vec![sequenced, unsequenced]);
        ingest_damage_events(&mut state, &mut inbox);
        assert!(inbox.is_empty());
        assert_eq!(
            state.pending_damage_retractions,
            vec![sequenced, unsequenced]
        );
        assert_eq!(state.applied_health_duplicates(), 0);
    }
}
