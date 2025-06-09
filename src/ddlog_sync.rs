use bevy::prelude::*;

use crate::components::{DdlogId, Health, Target, UnitType};
use crate::ddlog_handle::{DdlogEntity, DdlogHandle};

/// Pushes the current ECS state into DDlog.
/// This implementation is a stub that simply logs the state.
pub fn push_state_to_ddlog_system(
    mut ddlog: ResMut<DdlogHandle>,
    query: Query<(&DdlogId, &Transform, &Health, &UnitType, Option<&Target>)>,
) {
    use hashbrown::HashSet;

    let mut seen: HashSet<i64> = HashSet::with_capacity(ddlog.entities.len());

    for (id, transform, health, unit, target) in &query {
        seen.insert(id.0);
        ddlog
            .entities
            .entry(id.0)
            .and_modify(|e| {
                e.position = transform.translation.truncate();
                e.unit = unit.clone();
                e.health = health.0;
                e.target = target.map(|t| **t);
            })
            .or_insert_with(|| DdlogEntity {
                position: transform.translation.truncate(),
                unit: unit.clone(),
                health: health.0,
                target: target.map(|t| **t),
            });
    }

    ddlog.entities.retain(|id, _| seen.contains(id));
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
