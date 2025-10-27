//! Behavioural streams deriving movement from fear and targets.
//!
//! These helpers merge fear levels with positions, transform targets into
//! movement decisions and apply those decisions to base positions.

use dbsp::{typed_batch::OrdZSet, RootCircuit, Stream};
use glam::DVec2;
use log::warn;
use ordered_float::OrderedFloat;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

use crate::FEAR_THRESHOLD;

use crate::dbsp_circuit::{FearLevel, MovementDecision, Position, Target};

trait StreamConcat {
    fn concat(&self, other: &Self) -> Self;
}

impl StreamConcat for Stream<RootCircuit, OrdZSet<FearLevel>> {
    fn concat(&self, other: &Self) -> Self {
        self.plus(other)
    }
}

/// Merges explicit fear inputs with entity positions, defaulting to zero.
///
/// Each position yields a [`FearLevel`] record. Explicit fear levels flow
/// through unchanged, while an antijoin identifies missing entities and assigns
/// them a `0.0` level before the results are unioned back together.
///
/// # Examples
/// ```rust,no_run
/// # use anyhow::Result;
/// # use dbsp::RootCircuit;
/// # use ordered_float::OrderedFloat;
/// # use lille::dbsp_circuit::{FearLevel, Position};
/// # use lille::dbsp_circuit::fear_level_stream;
/// # fn main() -> Result<()> {
/// let (mut circuit, (positions_in, fears_in, mut level_out)) =
///     RootCircuit::build(|circuit| {
///         let (positions_stream, positions_handle) =
///             circuit.add_input_zset::<Position>();
///         let (fears_stream, fears_handle) =
///             circuit.add_input_zset::<FearLevel>();
///         let handle =
///             fear_level_stream(&positions_stream, &fears_stream).output();
///         Ok((positions_handle, fears_handle, handle))
///     })?;
///
/// positions_in.push(
///     Position {
///         entity: 7,
///         x: 0.0.into(),
///         y: 0.0.into(),
///         z: 0.0.into(),
///     },
///     1,
/// );
/// circuit.step()?;
///
/// let levels: Vec<FearLevel> = level_out
///     .consolidate()
///     .iter()
///     .map(|(level, (), _)| level.clone())
///     .collect();
/// assert_eq!(
///     levels,
///     vec![FearLevel {
///         entity: 7,
///         level: OrderedFloat(0.0),
///     }]
/// );
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn fear_level_stream(
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
    fears: &Stream<RootCircuit, OrdZSet<FearLevel>>,
) -> Stream<RootCircuit, OrdZSet<FearLevel>> {
    let explicit = fears.clone();

    let missing = positions
        .map_index(|p| (p.entity, ()))
        .antijoin(&explicit.map_index(|f| (f.entity, ())))
        .map(|(entity, ())| FearLevel {
            entity: *entity,
            level: OrderedFloat(0.0),
        });

    explicit.concat(&missing)
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    SizeOf,
)]
#[archive_attr(derive(Ord, PartialOrd, Eq, PartialEq, Hash))]
struct PositionTarget {
    entity: i64,
    px: OrderedFloat<f64>,
    py: OrderedFloat<f64>,
    tx: OrderedFloat<f64>,
    ty: OrderedFloat<f64>,
}

#[inline]
fn should_flee(level: OrderedFloat<f64>) -> bool {
    level.into_inner() > FEAR_THRESHOLD
}

/// Threshold below which displacement is treated as zero when normalising.
///
/// The value `1e-12` avoids division by near-zero magnitudes. It suppresses
/// floating-point noise while remaining negligible for typical movement
/// distances.
const MIN_DIRECTION_MAGNITUDE: f64 = 1e-12;

fn decide_movement(level: OrderedFloat<f64>, pt: &PositionTarget) -> MovementDecision {
    let displacement = DVec2::new(
        pt.tx.into_inner() - pt.px.into_inner(),
        pt.ty.into_inner() - pt.py.into_inner(),
    );
    let scaled = displacement * if should_flee(level) { -1.0 } else { 1.0 };
    let magnitude = scaled.length();
    let direction = if magnitude > MIN_DIRECTION_MAGNITUDE {
        scaled / magnitude
    } else {
        DVec2::ZERO
    };

    MovementDecision {
        entity: pt.entity,
        dx: OrderedFloat(direction.x),
        dy: OrderedFloat(direction.y),
    }
}

/// Converts fear levels and targets into simple movement decisions.
///
/// Entities with a target move one unit towards it when their fear is below
/// [`FEAR_THRESHOLD`]; otherwise, they flee one unit away. Vectors are
/// normalised to ensure consistent speed in all directions.
///
/// # Examples
/// ```rust,no_run
/// # use anyhow::Result;
/// # use dbsp::RootCircuit;
/// # use ordered_float::OrderedFloat;
/// # use lille::dbsp_circuit::{FearLevel, MovementDecision, Position, Target};
/// # use lille::dbsp_circuit::{fear_level_stream, movement_decision_stream};
/// # use std::f64::consts::FRAC_1_SQRT_2;
/// # fn main() -> Result<()> {
/// let (mut circuit, (fear_in, target_in, pos_in, mut decisions_out)) =
///     RootCircuit::build(|circuit| {
///         let (fear_stream, fear_handle) =
///             circuit.add_input_zset::<FearLevel>();
///         let (target_stream, target_handle) =
///             circuit.add_input_zset::<Target>();
///         let (position_stream, position_handle) =
///             circuit.add_input_zset::<Position>();
///
///         let fear = fear_level_stream(&position_stream, &fear_stream);
///         let output = movement_decision_stream(
///             &fear,
///             &target_stream,
///             &position_stream,
///         )
///         .output();
///         Ok((fear_handle, target_handle, position_handle, output))
///     })?;
///
/// fear_in.push(
///     FearLevel {
///         entity: 1,
///         level: OrderedFloat(0.1),
///     },
///     1,
/// );
/// target_in.push(
///     Target {
///         entity: 1,
///         x: 1.0.into(),
///         y: 1.0.into(),
///     },
///     1,
/// );
/// pos_in.push(
///     Position {
///         entity: 1,
///         x: 0.0.into(),
///         y: 0.0.into(),
///         z: 0.0.into(),
///     },
///     1,
/// );
/// circuit.step()?;
///
/// let decisions: Vec<MovementDecision> = decisions_out
///     .consolidate()
///     .iter()
///     .map(|(decision, (), _)| decision.clone())
///     .collect();
/// assert_eq!(decisions.len(), 1);
/// let decision = &decisions[0];
/// assert_eq!(decision.entity, 1);
/// assert_eq!(decision.dx, OrderedFloat(FRAC_1_SQRT_2));
/// assert_eq!(decision.dy, OrderedFloat(FRAC_1_SQRT_2));
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn movement_decision_stream(
    fear: &Stream<RootCircuit, OrdZSet<FearLevel>>,
    targets: &Stream<RootCircuit, OrdZSet<Target>>,
    positions: &Stream<RootCircuit, OrdZSet<Position>>,
) -> Stream<RootCircuit, OrdZSet<MovementDecision>> {
    let pos_target = positions
        .map_index(|p| (p.entity, *p))
        .join(&targets.map_index(|t| (t.entity, *t)), |_entity, p, t| {
            PositionTarget {
                entity: p.entity,
                px: p.x,
                py: p.y,
                tx: t.x,
                ty: t.y,
            }
        })
        .map_index(|pt| (pt.entity, pt.clone()));

    fear.map_index(|f| (f.entity, f.level))
        .join(&pos_target, |_entity, &level, pt| {
            decide_movement(level, pt)
        })
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use dbsp::RootCircuit;
    use rstest::rstest;

    fn pt(px: f64, py: f64, tx: f64, ty: f64) -> PositionTarget {
        PositionTarget {
            entity: 1,
            px: px.into(),
            py: py.into(),
            tx: tx.into(),
            ty: ty.into(),
        }
    }

    #[rstest]
    #[case::approach(0.1, std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2)]
    #[case::flee(
        0.3,
        -std::f64::consts::FRAC_1_SQRT_2,
        -std::f64::consts::FRAC_1_SQRT_2
    )]
    #[case::equals_threshold(
        FEAR_THRESHOLD,
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2
    )]
    fn decide_movement_direction(
        #[case] fear: f64,
        #[case] expected_dx: f64,
        #[case] expected_dy: f64,
    ) {
        let mv = decide_movement(fear.into(), &pt(0.0, 0.0, 1.0, 1.0));
        assert_relative_eq!(mv.dx.into_inner(), expected_dx);
        assert_relative_eq!(mv.dy.into_inner(), expected_dy);
    }

    #[rstest]
    #[case::approach(
        Some(0.1),
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2
    )]
    #[case::flee(
        Some(0.3),
        -std::f64::consts::FRAC_1_SQRT_2,
        -std::f64::consts::FRAC_1_SQRT_2
    )]
    #[case::equals_threshold(
        Some(FEAR_THRESHOLD),
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2
    )]
    #[case::default(None, std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2)]
    fn movement_decision_join(
        #[case] fear: Option<f64>,
        #[case] expected_dx: f64,
        #[case] expected_dy: f64,
    ) {
        let (circuit, (fear_in, target_in, pos_in, decisions_handle)) = RootCircuit::build(|c| {
            let (fear_input, fi) = c.add_input_zset::<FearLevel>();
            let (targets, ti) = c.add_input_zset::<Target>();
            let (pos_s, pi) = c.add_input_zset::<Position>();
            let fear_stream = fear_level_stream(&pos_s, &fear_input);
            let output_handle = movement_decision_stream(&fear_stream, &targets, &pos_s).output();
            Ok((fi, ti, pi, output_handle))
        })
        .expect("failed to build circuit for movement_decision_join");

        if let Some(level) = fear {
            fear_in.push(
                FearLevel {
                    entity: 1,
                    level: level.into(),
                },
                1,
            );
        }
        target_in.push(
            Target {
                entity: 1,
                x: 1.0.into(),
                y: 1.0.into(),
            },
            1,
        );
        pos_in.push(
            Position {
                entity: 1,
                x: 0.0.into(),
                y: 0.0.into(),
                z: 0.0.into(),
            },
            1,
        );

        circuit.step().expect("dbsp step");

        let decisions: Vec<MovementDecision> = decisions_handle
            .consolidate()
            .iter()
            .map(|(decision, (), _timestamp)| decision)
            .collect();
        let decision = test_utils::expect_single(&decisions, "movement decision result");
        assert_relative_eq!(decision.dx.into_inner(), expected_dx);
        assert_relative_eq!(decision.dy.into_inner(), expected_dy);
    }

    #[test]
    fn no_decision_without_target() {
        #[expect(
            unused_mut,
            reason = "Mutable binding retained for compatibility with API expectations"
        )]
        let (mut circuit, (fear_in, pos_in, decisions_handle)) = RootCircuit::build(|c| {
            let (fear_input, fi) = c.add_input_zset::<FearLevel>();
            let (pos_s, pi) = c.add_input_zset::<Position>();
            let targets = c.add_input_zset::<Target>().0;
            let fear_stream = fear_level_stream(&pos_s, &fear_input);
            let output_handle = movement_decision_stream(&fear_stream, &targets, &pos_s).output();
            Ok((fi, pi, output_handle))
        })
        .expect("failed to build circuit for no_decision_without_target");

        fear_in.push(
            FearLevel {
                entity: 1,
                level: 0.0.into(),
            },
            1,
        );
        pos_in.push(
            Position {
                entity: 1,
                x: 0.0.into(),
                y: 0.0.into(),
                z: 0.0.into(),
            },
            1,
        );

        circuit.step().expect("dbsp step");

        let decisions: Vec<MovementDecision> = decisions_handle
            .consolidate()
            .iter()
            .map(|(decision, (), _timestamp)| decision)
            .collect();
        assert!(decisions.is_empty());
    }
}
