//! Systems for synchronizing ECS state with `DDlog`.
//! Pushes component data to the database and applies deltas back to the world.
use bevy::prelude::*;
use hashbrown::HashMap;

use crate::components::{Block, BlockSlope, DdlogId, Health, Target, UnitType};
use crate::ddlog_handle::{DdlogEntity, DdlogHandle};

#[cfg(feature = "ddlog")]
#[allow(unused_imports)]
use differential_datalog::{DDlog, DDlogDynamic};
#[cfg(feature = "ddlog")]
use ordered_float::OrderedFloat;

/// Pushes the current ECS state into DDlog.
/// This implementation is a stub that simply logs the state.
pub fn push_state_to_ddlog_system(
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

    #[cfg(feature = "ddlog")]
    {
        use differential_datalog::record::{IntoRecord, RelIdentifier, UpdCmd};
        use lille_ddlog::{typedefs::entity_state::Position, Relations};

        let mut cmds = Vec::new();
        for (&id, ent) in ddlog.entities.iter() {
            let record = Position {
                entity: id,
                x: OrderedFloat(ent.position.x),
                y: OrderedFloat(ent.position.y),
                z: OrderedFloat(ent.position.z),
            };
            cmds.push(UpdCmd::Insert(
                RelIdentifier::RelId(Relations::entity_state_Position as usize),
                record.into_record(),
            ));
        }
        if let Some(prog) = ddlog.prog.as_mut() {
            if let Err(e) = prog.transaction_start() {
                log::error!("DDlog transaction_start failed: {e}");
            } else {
                let mut iter = cmds.into_iter();
                if let Err(e) = prog.apply_updates_dynamic(&mut iter) {
                    log::error!("DDlog apply_updates failed: {e}");
                }
            }
        }
    }
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
