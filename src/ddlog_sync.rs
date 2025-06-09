use bevy::prelude::*;

use crate::components::{DdlogId, Health, Target, UnitType};
use crate::ddlog_handle::{DdlogEntity, DdlogHandle};

/// Pushes the current ECS state into DDlog.
/// This implementation is a stub that simply logs the state.
pub fn push_state_to_ddlog_system(
    mut ddlog: ResMut<DdlogHandle>,
    query: Query<(&DdlogId, &Transform, &Health, &UnitType, Option<&Target>)>,
) {
    ddlog.entities.clear();
    for (id, transform, health, unit, target) in &query {
        ddlog.entities.insert(
            id.0,
            DdlogEntity {
                position: transform.translation.truncate(),
                unit: unit.clone(),
                health: health.0,
                target: target.map(|t| **t),
            },
        );
    }
}

/// Applies the inferred movement deltas from the DDlog stub.
pub fn apply_ddlog_deltas_system(
    mut ddlog: ResMut<DdlogHandle>,
    mut query: Query<(&DdlogId, &mut Transform)>,
) {
    let baddies: Vec<(Vec2, f32)> = ddlog
        .entities
        .values()
        .filter_map(|e| match e.unit {
            UnitType::Baddie { meanness } => Some((e.position, meanness)),
            _ => None,
        })
        .collect();

    for (id, mut transform) in &mut query {
        if let Some(ent) = ddlog.entities.get_mut(&id.0) {
            if let UnitType::Civvy { fraidiness } = ent.unit {
                let mut fear_vector = Vec2::ZERO;
                for (b_pos, meanness) in &baddies {
                    let to_actor = ent.position - *b_pos;
                    let distance = to_actor.length();
                    let speed = 5.0;
                    let fear_radius = fraidiness * *meanness * 2.0;
                    let avoidance_radius = fear_radius.max(speed);
                    if distance <= avoidance_radius {
                        let perp = Vec2::new(-to_actor.y, to_actor.x).normalize_or_zero();
                        let fear_scale = if distance < fear_radius {
                            ((fear_radius - distance) / fear_radius).powi(2) * 5.0
                        } else {
                            distance / avoidance_radius
                        };
                        fear_vector +=
                            to_actor.normalize_or_zero() * fear_scale + perp * fear_scale;
                    }
                }

                let target = ent.target.unwrap_or(ent.position);
                let to_target = target - ent.position;
                let target_dir = to_target.normalize_or_zero();
                let fear_dir = fear_vector.normalize_or_zero();
                let fear_influence = fear_vector.length();
                let fear_weight = (fear_influence * 2.0).min(1.0);
                let target_weight = 1.0 - fear_weight * 0.8;
                let move_vec = fear_dir * fear_weight + target_dir * target_weight;
                let final_dir = if move_vec.length_squared() > 0.0 {
                    move_vec.normalize() * 5.0
                } else {
                    Vec2::ZERO
                };
                ent.position += final_dir;
                transform.translation.x = ent.position.x;
                transform.translation.y = ent.position.y;
            }
        }
    }
}
