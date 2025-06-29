//! Systems for synchronizing ECS state with `DDlog`.
//! Pushes component data to the database and applies deltas back to the world.
use bevy::prelude::*;
use hashbrown::HashMap;

use crate::components::{Block, BlockSlope, DdlogId, Health, Target, UnitType};
use crate::ddlog_handle::{DdlogEntity, DdlogHandle};

/// Caches the current ECS state on [`DdlogHandle`].
///
/// This system no longer interacts with the DDlog runtime directly. It merely
/// mirrors relevant component data into the [`DdlogHandle`] resource so that
/// [`DdlogHandle::step`](crate::ddlog_handle::DdlogHandle::step) can process it
/// later.
pub fn cache_state_for_ddlog_system(
    mut ddlog: ResMut<DdlogHandle>,
    entity_query: Query<(&DdlogId, &Transform, &Health, &UnitType, Option<&Target>)>,
    block_query: Query<(&Block, Option<&BlockSlope>)>,
) {
    let mut new_entities = HashMap::with_capacity(entity_query.iter().len());
    let mut blocks = Vec::with_capacity(block_query.iter().len());
    let mut slopes = HashMap::with_capacity(block_query.iter().len());

    for (block, slope) in &block_query {
        blocks.push(block.clone());
        if let Some(s) = slope {
            slopes.insert(block.id, s.clone());
        }
    }

    for (id, transform, health, unit, target) in &entity_query {
        log::trace!(
            "Sync Entity {} pos=({:.1},{:.1}) hp={} unit={:?} has_target={}",
            id.0,
            transform.translation.x,
            transform.translation.y,
            health.0,
            unit,
            target.is_some()
        );

        new_entities.insert(
            id.0,
            DdlogEntity {
                position: transform.translation,
                unit: unit.clone(),
                health: health.0,
                target: target.map(|t| **t),
            },
        );
    }

    ddlog.entities = new_entities;
    ddlog.blocks = blocks;
    ddlog.slopes = slopes;
}

/// Applies the inferred movement deltas from the DDlog stub.
pub fn apply_ddlog_deltas_system(
    mut ddlog: ResMut<DdlogHandle>,
    mut query: Query<(&DdlogId, &mut Transform)>,
) {
    ddlog.step();

    for (id, mut transform) in &mut query {
        if let Some(ent) = ddlog.entities.get(&id.0) {
            transform.translation = ent.position;
        }
    }
}
