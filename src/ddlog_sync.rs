use bevy::prelude::*;

use crate::components::{DdlogId, Health, Target, UnitType};
use crate::ddlog_handle::DdlogHandle;

/// Pushes the current ECS state into DDlog.
/// This implementation is a stub that simply logs the state.
pub fn push_state_to_ddlog_system(
    _ddlog: Res<DdlogHandle>,
    query: Query<(&DdlogId, &Transform, &Health, &UnitType, Option<&Target>)>,
) {
    for (id, transform, health, unit, target) in &query {
        info!(
            "Sync Entity {} pos=({:.1},{:.1}) hp={} type={:?} has_target={}",
            id.0,
            transform.translation.x,
            transform.translation.y,
            health.0,
            unit,
            target.is_some()
        );
    }
}
