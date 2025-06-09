use bevy::prelude::*;
use hashbrown::HashMap;

use crate::components::{DdlogId, Health, Target, UnitType};
use crate::ddlog_handle::{DdlogEntity, DdlogHandle};

/// Pushes the current ECS state into DDlog.
/// This implementation is a stub that simply logs the state.
pub fn push_state_to_ddlog_system(
    mut ddlog: ResMut<DdlogHandle>,
    query: Query<(&DdlogId, &Transform, &Health, &UnitType, Option<&Target>)>,
) {
    let mut new_entities = HashMap::with_capacity(query.iter().len());

    for (id, transform, health, unit, target) in &query {
        log::trace!(
            "Sync Entity {} pos=({:.1},{:.1}) hp={} type={:?} has_target={}",
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
                position: transform.translation.truncate(),
                unit: unit.clone(),
                health: health.0,
                target: target.map(|t| **t),
            },
        );
    }

    ddlog.entities = new_entities;
}

/// Applies the inferred movement deltas from the DDlog stub.
pub fn apply_ddlog_deltas_system(
    mut ddlog: ResMut<DdlogHandle>,
    mut query: Query<(&DdlogId, &mut Transform)>,
) {
    ddlog.infer_movement();

    for (id, mut transform) in &mut query {
        if let Some(ent) = ddlog.entities.get(&id.0) {
            transform.translation.x = ent.position.x;
            transform.translation.y = ent.position.y;
        }
    }
}
