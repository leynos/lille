//! Internal helpers shared across the DBSP circuit implementation.

use crate::GRACE_DISTANCE;

use super::{PositionFloor, Tick};

/// Determines whether the entity is within the configured grace distance.
#[must_use]
pub(crate) fn within_grace(pf: &PositionFloor) -> bool {
    pf.position.z.into_inner() <= pf.z_floor.into_inner() + GRACE_DISTANCE
}

/// Advances the tick counter, resetting to zero on overflow.
#[must_use]
pub(crate) fn advance_tick(tick: &mut Tick) -> Tick {
    let current = *tick;
    *tick = current.checked_add(1).map_or_else(
        || {
            debug_assert!(false, "tick counter overflowed u64");
            0
        },
        |next| next,
    );
    current
}
