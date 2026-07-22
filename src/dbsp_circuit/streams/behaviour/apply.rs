//! Application of movement decisions to base positions.
//!
//! Joins movement decisions with base positions to produce moved positions,
//! passing unmoved entities through unchanged.

use dbsp::{typed_batch::OrdZSet, RootCircuit, Stream};
use log::warn;
use ordered_float::OrderedFloat;

use super::decide::dedupe_movement_decisions;
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
    // Fold duplicate decisions per entity into a single movement before the
    // join, mirroring the decision stream's own dedupe. This keeps the join
    // from applying a doubled delta when upstream emits more than one record
    // for an entity in a tick. The deduped stream feeds both the duplicate
    // validation below and the join.
    let deduped = dedupe_movement_decisions(movement);
    let mv_base = deduped.map_index(|m| (m.entity, (m.dx, m.dy)));

    let mv = mv_base.inspect(|batch| {
        // Accumulate counts per entity to catch duplicates surviving the
        // dedupe above. Any duplicate here indicates a bug; release builds log
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

#[cfg(test)]
mod tests {
    //! End-to-end tests for [`apply_movement`].
    //!
    //! Each test drives a minimal circuit, feeding base positions and movement
    //! decisions, then asserts the consolidated positions the stream emits.

    use super::apply_movement;
    use crate::dbsp_circuit::{MovementDecision, Position};
    use approx::{assert_relative_eq, relative_eq};
    use dbsp::RootCircuit;
    use ordered_float::OrderedFloat;

    type ApplyCircuit = (
        dbsp::CircuitHandle,
        (
            dbsp::ZSetHandle<Position>,
            dbsp::ZSetHandle<MovementDecision>,
            dbsp::OutputHandle<dbsp::typed_batch::OrdZSet<Position>>,
        ),
    );

    fn build_apply_circuit() -> Result<ApplyCircuit, dbsp::Error> {
        RootCircuit::build(|circuit| {
            let (base_stream, base_handle) = circuit.add_input_zset::<Position>();
            let (movement_stream, movement_handle) = circuit.add_input_zset::<MovementDecision>();
            let output_handle = apply_movement(&base_stream, &movement_stream).output();
            Ok((base_handle, movement_handle, output_handle))
        })
    }

    fn position_at(entity: i64, x: f64, y: f64, z: f64) -> Position {
        Position {
            entity,
            x: x.into(),
            y: y.into(),
            z: z.into(),
        }
    }

    fn position(entity: i64, x: f64, y: f64) -> Position {
        position_at(entity, x, y, 0.0)
    }

    fn movement(entity: i64, dx: f64, dy: f64) -> MovementDecision {
        MovementDecision {
            entity,
            dx: OrderedFloat(dx),
            dy: OrderedFloat(dy),
        }
    }

    fn collect_positions(
        handle: &dbsp::OutputHandle<dbsp::typed_batch::OrdZSet<Position>>,
    ) -> Vec<(Position, i64)> {
        handle
            .consolidate()
            .iter()
            .map(|(position, (), weight)| {
                let position_ref: &Position = &position;
                (*position_ref, weight)
            })
            .collect()
    }

    /// Extracts the sole emitted position, panicking if there is not exactly
    /// one (which keeps the tests free of panicking slice indexing).
    fn single_position(positions: &[(Position, i64)]) -> (Position, i64) {
        *test_utils::expect_single(positions, "expected exactly one output position")
    }

    #[test]
    fn applies_or_preserves_positions() {
        struct Case {
            name: &'static str,
            base: Position,
            movement: Option<MovementDecision>,
            expected: Position,
        }

        let cases = [
            // A targeted entity shifts by its decision's delta in x/y while its
            // z coordinate is carried through unchanged.
            Case {
                name: "applies movement to targeted entity",
                base: position_at(1, 0.0, 0.0, 2.0),
                movement: Some(movement(1, 1.0, 0.0)),
                expected: position_at(1, 1.0, 0.0, 2.0),
            },
            // Without a decision, the base position passes through unchanged.
            Case {
                name: "passes unmoved entity through",
                base: position_at(2, 5.0, 5.0, 3.0),
                movement: None,
                expected: position_at(2, 5.0, 5.0, 3.0),
            },
        ];

        for case in cases {
            let (circuit, (base_in, movement_in, out)) =
                build_apply_circuit().expect("failed to build apply circuit");
            base_in.push(case.base, 1);
            if let Some(decision) = case.movement {
                movement_in.push(decision, 1);
            }

            circuit.step().expect("dbsp step");

            let (actual, weight) = single_position(&collect_positions(&out));
            assert_eq!(weight, 1, "{}: output weight", case.name);
            assert_eq!(actual.entity, case.expected.entity, "{}: entity", case.name);
            assert!(
                relative_eq!(actual.x.into_inner(), case.expected.x.into_inner()),
                "{}: x (expected {}, got {})",
                case.name,
                case.expected.x.into_inner(),
                actual.x.into_inner()
            );
            assert!(
                relative_eq!(actual.y.into_inner(), case.expected.y.into_inner()),
                "{}: y (expected {}, got {})",
                case.name,
                case.expected.y.into_inner(),
                actual.y.into_inner()
            );
            assert!(
                relative_eq!(actual.z.into_inner(), case.expected.z.into_inner()),
                "{}: z (expected {}, got {}) — movement must preserve z",
                case.name,
                case.expected.z.into_inner(),
                actual.z.into_inner()
            );
        }
    }

    #[test]
    fn dedupes_duplicate_movement_decisions() {
        let (circuit, (base_in, movement_in, out)) =
            build_apply_circuit().expect("failed to build apply circuit");
        base_in.push(position(1, 0.0, 0.0), 1);
        // Two decisions for the same entity in one tick must collapse into one
        // normalised movement rather than doubling the applied delta.
        movement_in.push(movement(1, 2.0, 0.0), 1);
        movement_in.push(movement(1, 2.0, 0.0), 1);

        circuit.step().expect("dbsp step");

        // Exactly one output position, emitted once (weight 1): the join must
        // not see two decisions, and (2, 0) normalises to the unit vector (1, 0).
        let (moved, weight) = single_position(&collect_positions(&out));
        assert_eq!(weight, 1);
        assert_eq!(moved.entity, 1);
        assert_relative_eq!(moved.x.into_inner(), 1.0);
        assert_relative_eq!(moved.y.into_inner(), 0.0);
    }
}
