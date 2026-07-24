//! Movement-decision streams derived from fear levels and targets.
//!
//! Joins positions with targets, decides whether an entity approaches or
//! flees its target based on fear, and dedupes decisions per entity.

use dbsp::{algebra::Semigroup, operator::Fold, typed_batch::OrdZSet, RootCircuit, Stream};
use glam::DVec2;
use log::warn;
use ordered_float::OrderedFloat;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use size_of::SizeOf;

use crate::FEAR_THRESHOLD;

use crate::dbsp_circuit::{FearLevel, MovementDecision, Position, Target};

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
pub(super) struct PositionTarget {
    pub(super) entity: i64,
    pub(super) px: OrderedFloat<f64>,
    pub(super) py: OrderedFloat<f64>,
    pub(super) tx: OrderedFloat<f64>,
    pub(super) ty: OrderedFloat<f64>,
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

pub(super) fn decide_movement(level: OrderedFloat<f64>, pt: &PositionTarget) -> MovementDecision {
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

    let raw = fear
        .map_index(|f| (f.entity, f.level))
        .join(&pos_target, |_entity, &level, pt| {
            decide_movement(level, pt)
        });
    dedupe_movement_decisions(&raw)
}

/// Collapses per-tick movement decisions so each entity has at most one.
///
/// Decisions are indexed by entity and folded through [`MovementAccumulator`],
/// which sums each decision's `dx`/`dy` weighted by its Z-set weight and
/// tracks the total weight. The summed vector is then normalised back to a
/// unit direction; entities whose total weight nets to zero are dropped
/// entirely. This guarantees a single decision per entity downstream, so a
/// join cannot apply a doubled delta.
///
/// # Examples
/// ```text
/// let movement = movement_decision_stream(&fear, &targets, &positions);
/// let deduped = dedupe_movement_decisions(&movement);
/// // `deduped` carries at most one `MovementDecision` per entity per tick.
/// ```
fn dedupe_movement_decisions(
    movement: &Stream<RootCircuit, OrdZSet<MovementDecision>>,
) -> Stream<RootCircuit, OrdZSet<MovementDecision>> {
    movement
        .map_index(|decision| (decision.entity, *decision))
        .aggregate(Fold::<
            MovementDecision,
            MovementAccumulator,
            MovementAccumulatorSemigroup,
            _,
            _,
        >::with_output(
            MovementAccumulator::default(),
            |acc: &mut MovementAccumulator, decision: &MovementDecision, weight: i64| {
                acc.apply(decision, weight);
            },
            |acc: MovementAccumulator| acc,
        ))
        .flat_map(|(entity, accumulator)| accumulator.clone().into_decision(*entity).into_iter())
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    SizeOf,
)]
#[archive_attr(derive(Eq, PartialEq, Ord, PartialOrd, Hash))]
struct MovementAccumulator {
    sum_dx: OrderedFloat<f64>,
    sum_dy: OrderedFloat<f64>,
    total_weight: i64,
}

impl MovementAccumulator {
    fn apply(&mut self, movement: &MovementDecision, weight: i64) {
        let dx = movement.dx.into_inner();
        let dy = movement.dy.into_inner();
        #[expect(
            clippy::cast_precision_loss,
            reason = "Movement counts remain tiny so converting to f64 is exact"
        )]
        let scaled = weight as f64;
        self.sum_dx = OrderedFloat(self.sum_dx.into_inner() + dx * scaled);
        self.sum_dy = OrderedFloat(self.sum_dy.into_inner() + dy * scaled);
        self.total_weight += weight;
    }

    fn merge(&mut self, other: &Self) {
        self.sum_dx = OrderedFloat(self.sum_dx.into_inner() + other.sum_dx.into_inner());
        self.sum_dy = OrderedFloat(self.sum_dy.into_inner() + other.sum_dy.into_inner());
        self.total_weight += other.total_weight;
    }

    fn into_decision(self, entity: i64) -> Option<MovementDecision> {
        if self.total_weight == 0 {
            return None;
        }
        if self.total_weight.abs() > 1 {
            warn!(
                "aggregated {} movement decisions for entity {entity}, normalising to one vector",
                self.total_weight
            );
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "Movement counts remain tiny so converting to f64 is exact"
        )]
        let weight = self.total_weight as f64;
        let avg_x = self.sum_dx.into_inner() / weight;
        let avg_y = self.sum_dy.into_inner() / weight;
        let magnitude = (avg_x * avg_x + avg_y * avg_y).sqrt();
        let (dx, dy) = if magnitude > MIN_DIRECTION_MAGNITUDE {
            (avg_x / magnitude, avg_y / magnitude)
        } else {
            (0.0, 0.0)
        };
        Some(MovementDecision {
            entity,
            dx: OrderedFloat(dx),
            dy: OrderedFloat(dy),
        })
    }
}

#[derive(Clone)]
struct MovementAccumulatorSemigroup;

impl Semigroup<MovementAccumulator> for MovementAccumulatorSemigroup {
    fn combine(left: &MovementAccumulator, right: &MovementAccumulator) -> MovementAccumulator {
        let mut combined = left.clone();
        combined.merge(right);
        combined
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the private movement accumulator.
    use super::*;
    use approx::assert_relative_eq;

    fn decision(dx: f64, dy: f64) -> MovementDecision {
        MovementDecision {
            entity: 1,
            dx: OrderedFloat(dx),
            dy: OrderedFloat(dy),
        }
    }

    /// Order-sensitive cancellation: an east +1 contribution merged with a
    /// north -1 contribution nets the weight to zero, but the accumulated
    /// displacement must be preserved (an earlier reset-on-zero bug zeroed the
    /// sums). Re-adding north +1 then cancels the north component, leaving the
    /// original east direction. If the sums were reset on the net-zero merge,
    /// the final decision would wrongly point north.
    #[test]
    fn net_zero_merge_preserves_pending_direction() {
        // Axes: east = (+1, 0), north = (0, +1).
        let mut acc = MovementAccumulator::default();
        acc.apply(&decision(1.0, 0.0), 1); // east +1

        let mut north_retraction = MovementAccumulator::default();
        north_retraction.apply(&decision(0.0, 1.0), -1); // north -1

        acc.merge(&north_retraction);
        assert_eq!(
            acc.total_weight, 0,
            "east +1 and north -1 net to zero weight"
        );

        acc.apply(&decision(0.0, 1.0), 1); // north +1 restores unit weight

        let movement = acc
            .into_decision(1)
            .expect("net weight of one must yield a decision");
        assert_relative_eq!(movement.dx.into_inner(), 1.0);
        assert_relative_eq!(movement.dy.into_inner(), 0.0);
    }
}
