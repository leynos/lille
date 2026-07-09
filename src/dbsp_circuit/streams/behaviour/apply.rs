//! Application of movement decisions to base positions.
//!
//! Joins movement decisions with base positions to produce moved positions,
//! passing unmoved entities through unchanged.

use dbsp::{typed_batch::OrdZSet, RootCircuit, Stream};
use log::warn;
use ordered_float::OrderedFloat;

use crate::dbsp_circuit::{MovementDecision, Position};

/// Applies movement decisions to base positions.
///
/// Panics in debug builds if more than one movement record exists for the same
/// entity in a single tick.
///
/// # Examples
/// ```rust,no_run
/// # use anyhow::Result;
/// # use dbsp::RootCircuit;
/// # use ordered_float::OrderedFloat;
/// # use lille::dbsp_circuit::{MovementDecision, Position};
/// # use lille::dbsp_circuit::apply_movement;
/// # fn main() -> Result<()> {
/// let (mut circuit, (base_in, movement_in, mut moved_out)) =
///     RootCircuit::build(|circuit| {
///         let (base_stream, base_handle) =
///             circuit.add_input_zset::<Position>();
///         let (movement_stream, movement_handle) =
///             circuit.add_input_zset::<MovementDecision>();
///         let output =
///             apply_movement(&base_stream, &movement_stream).output();
///         Ok((base_handle, movement_handle, output))
///     })?;
///
/// base_in.push(
///     Position {
///         entity: 1,
///         x: 0.0.into(),
///         y: 0.0.into(),
///         z: 0.0.into(),
///     },
///     1,
/// );
/// movement_in.push(
///     MovementDecision {
///         entity: 1,
///         dx: OrderedFloat(1.0),
///         dy: OrderedFloat(0.0),
///     },
///     1,
/// );
/// circuit.step()?;
///
/// let moved: Vec<Position> = moved_out
///     .consolidate()
///     .iter()
///     .map(|(position, (), _)| position.clone())
///     .collect();
/// assert_eq!(moved.len(), 1);
/// assert_eq!(moved[0].entity, 1);
/// assert_eq!(moved[0].x, OrderedFloat(1.0));
/// assert_eq!(moved[0].y, OrderedFloat(0.0));
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn apply_movement(
    base: &Stream<RootCircuit, OrdZSet<Position>>,
    movement: &Stream<RootCircuit, OrdZSet<MovementDecision>>,
) -> Stream<RootCircuit, OrdZSet<Position>> {
    let base_idx = base.map_index(|p| (p.entity, *p));
    let mv_base = movement.map_index(|m| (m.entity, (m.dx, m.dy)));

    let mv = mv_base.inspect(|batch| {
        // Accumulate counts per entity to catch duplicates emitted within a
        // single tick. Duplicates indicate a bug upstream; release builds log
        // the issue while debug builds panic for visibility.
        use std::collections::HashMap;

        let mut counts: HashMap<i64, i64> = HashMap::new();
        for (entity, _mv, weight) in batch.iter() {
            *counts.entry(entity).or_default() += weight;
        }
        for (entity, total) in counts {
            if total > 1 {
                warn!("duplicate movement decisions for entity {entity}");
            }
            debug_assert!(
                total <= 1,
                "duplicate movement decisions for entity {entity}"
            );
        }
    });

    let moved = base_idx.join(&mv, |_, p, &(dx, dy)| Position {
        entity: p.entity,
        x: OrderedFloat(p.x.into_inner() + dx.into_inner()),
        y: OrderedFloat(p.y.into_inner() + dy.into_inner()),
        z: p.z,
    });
    let mv_entities = mv.map(|(e, _)| *e).map_index(|e| (*e, ()));
    let unmoved = base_idx.antijoin(&mv_entities).map(|(_, p)| *p);
    unmoved.plus(&moved)
}
