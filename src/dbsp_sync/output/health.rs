//! Health-delta application for the DBSP output system.
//!
//! Splits the health half of `apply_dbsp_outputs_system` out of
//! [`super`]: resolving a delta's target entity, honouring the
//! dedupe/retraction rules, clamping the result, and writing it back to the
//! `Health` component and the world handle.
//!
//! [`clamped_health_value`] is deliberately pure — it computes the new value
//! and reports whether clamping happened, leaving the logging to
//! [`apply_one_health_delta`] in the command layer.

use std::convert::TryFrom;

use bevy::prelude::*;
use log::{debug, warn};

use crate::components::Health;
use crate::dbsp_circuit::{HealthDelta, Tick};
use crate::world_handle::WorldHandle;

use super::{DbspState, DbspWriteQuery};

/// Resolves the entity key and mutable `Health` component for a health delta.
///
/// Returns `None` (after logging where appropriate) when the entity id cannot
/// be mapped, the entity is missing from the query, or it carries no `Health`
/// component. Extracting this guard keeps [`apply_health_deltas`] within the
/// cognitive-complexity budget without suppressing the lint.
pub(super) fn resolve_health_target<'q>(
    state: &DbspState,
    write_query: &'q mut DbspWriteQuery<'_, '_>,
    delta: &HealthDelta,
) -> Option<(i64, Mut<'q, Health>)> {
    let Ok(entity_key) = i64::try_from(delta.entity) else {
        warn!("health delta for unmappable entity {}", delta.entity);
        return None;
    };
    let Some(&entity) = state.id_map.get(&entity_key) else {
        warn!("health delta for unknown entity id {}", delta.entity);
        return None;
    };
    let Ok((_, _, _, maybe_health)) = write_query.get_mut(entity) else {
        return None;
    };
    let Some(health) = maybe_health else {
        warn!("health delta received for entity without Health component");
        return None;
    };
    Some((entity_key, health))
}

/// Returns the number of deltas skipped for a non-positive Z-set weight.
pub(super) fn apply_health_deltas(
    state: &mut DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
) -> u64 {
    let mut skipped = 0;
    let health_deltas = state.circuit.health_delta_out().consolidate();
    for (delta, (), weight) in health_deltas.iter() {
        // Apply only positive weights; skip retractions (negative) and
        // zero-weight deltas so they never mutate Health or update
        // applied_health/world_handle.
        if weight <= 0 {
            skipped += 1;
            continue;
        }
        apply_one_health_delta(state, write_query, world_handle, &delta);
    }
    skipped
}

/// Applies a single positive-weight health delta: resolves the target entity's
/// `Health`, honours the dedup/retraction rules, clamps the new value, then
/// updates the component, `applied_health` bookkeeping, and the world handle.
/// Any guard failing simply skips this delta.
pub(super) fn apply_one_health_delta(
    state: &mut DbspState,
    write_query: &mut DbspWriteQuery<'_, '_>,
    world_handle: &mut WorldHandle,
    delta: &HealthDelta,
) {
    let key = (delta.at_tick, delta.seq);
    let Some((entity_key, mut health)) = resolve_health_target(state, write_query, delta) else {
        return;
    };
    if !should_apply_health_delta(state, delta, key) {
        return;
    }
    let Some(clamped) = clamped_health_value(delta, &health) else {
        return;
    };
    // The pure helper reports clamping through its result; the log lives here,
    // in the command layer, and keeps the entity, raw, and clamped values.
    if let Some(raw) = clamped.clamped_from {
        debug!(
            "health clamped for entity {}: raw {} -> {}",
            delta.entity, raw, clamped.value
        );
    }
    health.current = clamped.value;
    state.applied_health.insert(delta.entity, key);
    if let Some(entry) = world_handle.entities.get_mut(&entity_key) {
        entry.health_current = health.current;
        entry.health_max = health.max;
    }
}

/// Outcome of clamping a health delta into the component's valid range.
///
/// Carrying the pre-clamp total back to the caller is what lets
/// [`clamped_health_value`] stay pure: the diagnostic is reported by
/// [`apply_one_health_delta`], not from inside the computation.
pub(super) struct ClampedHealth {
    /// The new `Health::current`, already within `0..=health.max`.
    pub(super) value: u16,
    /// `Some(raw)` when the uncapped total fell outside the valid range and
    /// was clamped; `None` when no clamping was needed.
    pub(super) clamped_from: Option<i32>,
}

/// Computes the clamped `Health::current` for a delta, returning `None` if the
/// clamped value cannot fit in `u16`.
///
/// Pure: it neither logs nor mutates. The caller reports any clamping using the
/// returned [`ClampedHealth::clamped_from`].
pub(super) fn clamped_health_value(delta: &HealthDelta, health: &Health) -> Option<ClampedHealth> {
    let current = i32::from(health.current);
    let max = i32::from(health.max);
    let raw = current + delta.delta;
    let new_value = raw.clamp(0, max);
    let clamped_from = (raw != new_value).then_some(raw);
    // `new_value` is `raw` clamped to `0..=health.max`, and `health.max` is a
    // `u16`, so the value is always within `u16` range and this conversion
    // cannot fail. The branch below is belt and braces against a future change
    // to the clamp bounds.
    let Ok(value) = u16::try_from(new_value) else {
        debug_assert!(
            false,
            "clamped health value {new_value} exceeds u16 capacity"
        );
        return None;
    };
    Some(ClampedHealth {
        value,
        clamped_from,
    })
}

pub(super) fn should_apply_health_delta(
    state: &mut DbspState,
    delta: &HealthDelta,
    key: (Tick, Option<u32>),
) -> bool {
    if state
        .expected_health_retractions
        .remove(&(delta.entity, delta.at_tick, delta.seq))
    {
        return false;
    }
    if state.applied_health.get(&delta.entity) == Some(&key) {
        debug!(
            "duplicate health delta ignored for entity {} at tick {} seq {:?}",
            delta.entity, delta.at_tick, delta.seq
        );
        state.health_duplicate_count += 1;
        return false;
    }
    true
}
