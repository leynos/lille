//! Unit tests for the private movement accumulator and dedupe stream.
use super::*;
use approx::assert_relative_eq;
use rstest::rstest;

fn decision(entity: i64, dx: f64, dy: f64) -> MovementDecision {
    MovementDecision {
        entity,
        dx: OrderedFloat(dx),
        dy: OrderedFloat(dy),
    }
}

/// One weighted input decision: which entity it targets, its direction, and
/// its Z-set weight.
type WeightedDecision = (i64, (f64, f64), i64);

/// Tags single-entity fixtures with entity 1, so tests that do not care about
/// the entity key stay readable.
fn for_entity_one(weighted: &[((f64, f64), i64)]) -> Vec<WeightedDecision> {
    weighted
        .iter()
        .map(|&(direction, weight)| (1, direction, weight))
        .collect()
}

/// Both streams [`dedupe_movement_decisions`] produces, each record paired
/// with its consolidated Z-set weight (multiplicity).
struct DedupeOutputs {
    decisions: Vec<(MovementDecision, i64)>,
    aggregations: Vec<(MovementAggregation, i64)>,
}

/// Runs a single entity's weighted decisions through
/// [`dedupe_movement_decisions`] and returns both output streams.
///
/// Returns the fallible `Result` (rather than unwrapping) so the circuit
/// construction stays outside a `no_expect_outside_tests` boundary; callers
/// unwrap it.
fn deduped_outputs(weighted: &[WeightedDecision]) -> Result<DedupeOutputs, dbsp::Error> {
    let (circuit, (input, decisions_out, aggregations_out)) = RootCircuit::build(|circuit| {
        let (stream, handle) = circuit.add_input_zset::<MovementDecision>();
        let (deduped, aggregations) = dedupe_movement_decisions(&stream);
        Ok((handle, deduped.output(), aggregations.output()))
    })?;
    for &(entity, (dx, dy), weight) in weighted {
        input.push(decision(entity, dx, dy), weight);
    }
    circuit.step()?;
    Ok(DedupeOutputs {
        decisions: test_utils::collect_weighted(&decisions_out),
        aggregations: test_utils::collect_weighted(&aggregations_out),
    })
}

/// Convenience wrapper for the tests that only assert on the decision stream.
fn deduped_decisions(
    weighted: &[((f64, f64), i64)],
) -> Result<Vec<(MovementDecision, i64)>, dbsp::Error> {
    Ok(deduped_outputs(&for_entity_one(weighted))?.decisions)
}

/// Bounded matrix over duplicate and cancelling weighted decisions for one
/// entity: a positive total weight yields exactly one normalized decision;
/// a net-zero total weight yields none.
#[rstest]
#[case::single_positive(&[((1.0, 0.0), 1)], Some((1.0, 0.0)))]
#[case::duplicate_positive(&[((2.0, 0.0), 1), ((2.0, 0.0), 1)], Some((1.0, 0.0)))]
#[case::weighted_positive(&[((0.0, 3.0), 2)], Some((0.0, 1.0)))]
#[case::cancel_to_zero(&[((1.0, 0.0), 1), ((1.0, 0.0), -1)], None)]
#[case::mixed_net_zero(
    &[((1.0, 0.0), 1), ((0.0, 1.0), 1), ((1.0, 0.0), -1), ((0.0, 1.0), -1)],
    None
)]
fn dedupe_emits_one_decision_for_positive_weight_and_none_for_zero(
    #[case] weighted: &[((f64, f64), i64)],
    #[case] expected: Option<(f64, f64)>,
) {
    let decisions = deduped_decisions(weighted).expect("dedupe circuit run");
    match expected {
        Some((expected_dx, expected_dy)) => {
            let (movement, weight) =
                test_utils::expect_single(&decisions, "positive weight must emit one decision");
            assert_eq!(
                *weight, 1,
                "a deduped decision must have Z-set multiplicity 1, not {weight}"
            );
            assert_relative_eq!(movement.dx.into_inner(), expected_dx);
            assert_relative_eq!(movement.dy.into_inner(), expected_dy);
        }
        None => assert!(
            decisions.is_empty(),
            "net-zero total weight must emit no decision, got {decisions:?}"
        ),
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
    acc.apply(&decision(1, 1.0, 0.0), 1); // east +1

    let mut north_retraction = MovementAccumulator::default();
    north_retraction.apply(&decision(1, 0.0, 1.0), -1); // north -1

    acc.merge(&north_retraction);
    assert_eq!(
        acc.total_weight, 0,
        "east +1 and north -1 net to zero weight"
    );

    acc.apply(&decision(1, 0.0, 1.0), 1); // north +1 restores unit weight

    let movement = acc
        .to_decision(1)
        .decision
        .expect("net weight of one must yield a decision");
    assert_relative_eq!(movement.dx.into_inner(), 1.0);
    assert_relative_eq!(movement.dy.into_inner(), 0.0);
}

/// `to_decision` is pure, so it can be exercised directly. This matrix pins
/// both halves of its result: the normalized decision and the aggregation
/// diagnostic that replaced the helper's former `warn!` call. The diagnostic
/// appears only when more than one decision was folded in, which is exactly
/// the condition the old log fired on.
#[rstest]
#[case::net_zero_yields_nothing(&[((1.0, 0.0), 1), ((1.0, 0.0), -1)], None, None)]
#[case::single_decision_is_not_aggregated(&[((3.0, 0.0), 1)], Some((1.0, 0.0)), None)]
#[case::duplicate_is_aggregated(&[((1.0, 0.0), 1), ((1.0, 0.0), 1)], Some((1.0, 0.0)), Some(2))]
#[case::weight_two_is_aggregated(&[((0.0, 2.0), 2)], Some((0.0, 1.0)), Some(2))]
#[case::net_negative_is_aggregated(&[((1.0, 0.0), -2)], Some((1.0, 0.0)), Some(-2))]
#[case::opposed_sum_below_threshold(
    &[((1.0, 0.0), 1), ((-1.0, 0.0), 1)],
    Some((0.0, 0.0)),
    Some(2)
)]
fn to_decision_reports_movement_and_aggregation(
    #[case] weighted: &[((f64, f64), i64)],
    #[case] expected_direction: Option<(f64, f64)>,
    #[case] expected_total_weight: Option<i64>,
) {
    let mut acc = MovementAccumulator::default();
    for &((dx, dy), weight) in weighted {
        acc.apply(&decision(1, dx, dy), weight);
    }
    let outcome = acc.to_decision(1);

    match expected_direction {
        Some((dx, dy)) => {
            let movement = outcome
                .decision
                .expect("a non-zero total weight must yield a decision");
            assert_eq!(movement.entity, 1);
            assert_relative_eq!(movement.dx.into_inner(), dx);
            assert_relative_eq!(movement.dy.into_inner(), dy);
        }
        None => assert!(
            outcome.decision.is_none(),
            "a net-zero total weight must yield no decision, got {:?}",
            outcome.decision
        ),
    }

    match expected_total_weight {
        Some(total_weight) => {
            let aggregation = outcome
                .aggregation
                .expect("folding several decisions must report an aggregation");
            assert_eq!(aggregation.entity, 1);
            assert_eq!(aggregation.total_weight, total_weight);
        }
        None => assert!(
            outcome.aggregation.is_none(),
            "a single decision must not report an aggregation, got {:?}",
            outcome.aggregation
        ),
    }
}

/// `to_decision` must not panic on the extremes of `i64`. `i64::MIN.abs()`
/// overflows, so the aggregation check compares against the bounds instead of
/// negating; these are the cases that would trip a naive `abs()`.
#[rstest]
#[case::min(i64::MIN)]
#[case::max(i64::MAX)]
#[case::minus_one(-1)]
#[case::one(1)]
fn to_decision_handles_extreme_total_weights(#[case] total_weight: i64) {
    let acc = MovementAccumulator {
        sum_dx: OrderedFloat(1.0),
        sum_dy: OrderedFloat(0.0),
        total_weight,
    };
    let outcome = acc.to_decision(1);
    assert!(
        outcome.decision.is_some(),
        "a non-zero total weight must yield a decision"
    );
    let expects_aggregation = !(-1..=1).contains(&total_weight);
    assert_eq!(
        outcome.aggregation.is_some(),
        expects_aggregation,
        "aggregation must be reported exactly when more than one decision folded in"
    );
}

/// The diagnostic reaches the circuit's own output stream, which is what
/// `apply_dbsp_outputs_system` reads to log aggregation. Asserting on the
/// stream (rather than on log text, for which the repository has no capture
/// helper) pins the command-side diagnostic path end to end.
#[rstest]
#[case::single_decision_emits_no_diagnostic(&[((1.0, 0.0), 1)], None)]
#[case::duplicate_emits_diagnostic(&[((1.0, 0.0), 1), ((1.0, 0.0), 1)], Some(2))]
#[case::net_zero_emits_no_diagnostic(&[((1.0, 0.0), 1), ((1.0, 0.0), -1)], None)]
fn dedupe_emits_aggregation_diagnostics(
    #[case] weighted: &[((f64, f64), i64)],
    #[case] expected_total_weight: Option<i64>,
) {
    let outputs = deduped_outputs(&for_entity_one(weighted)).expect("dedupe circuit run");
    match expected_total_weight {
        Some(total_weight) => {
            let (aggregation, weight) = test_utils::expect_single(
                &outputs.aggregations,
                "a collapsed entity must emit one diagnostic",
            );
            assert_eq!(*weight, 1, "the diagnostic must have multiplicity 1");
            assert_eq!(aggregation.entity, 1);
            assert_eq!(aggregation.total_weight, total_weight);
        }
        None => assert!(
            outputs.aggregations.is_empty(),
            "no aggregation occurred, so no diagnostic must be emitted, got {:?}",
            outputs.aggregations
        ),
    }
}

mod properties;

/// Deduplication is per entity, not global. Entity 1's two decisions collapse
/// into one; entity 2's single decision passes through untouched. Both
/// decisions must survive, and only entity 1 — the one actually collapsed —
/// may produce an aggregation diagnostic.
#[rstest]
fn dedupe_separates_entities() {
    // Entity 1 points east twice; entity 2 points north once.
    let outputs = deduped_outputs(&[(1, (1.0, 0.0), 1), (1, (1.0, 0.0), 1), (2, (0.0, 1.0), 1)])
        .expect("dedupe circuit run");

    let mut decisions = outputs.decisions.clone();
    decisions.sort_by_key(|(movement, _)| movement.entity);
    let [(first, first_weight), (second, second_weight)] = decisions.as_slice() else {
        panic!("both entities must emit a decision, got {decisions:?}");
    };

    assert_eq!(first.entity, 1);
    assert_eq!(
        *first_weight, 1,
        "the collapsed decision keeps multiplicity 1"
    );
    assert_relative_eq!(first.dx.into_inner(), 1.0);
    assert_relative_eq!(first.dy.into_inner(), 0.0);

    assert_eq!(second.entity, 2);
    assert_eq!(*second_weight, 1);
    assert_relative_eq!(second.dx.into_inner(), 0.0);
    assert_relative_eq!(second.dy.into_inner(), 1.0);

    // Only entity 1 aggregated, so entity 2 must not appear in the diagnostics.
    let (aggregation, weight) = test_utils::expect_single(
        &outputs.aggregations,
        "only the collapsed entity may emit a diagnostic",
    );
    assert_eq!(*weight, 1);
    assert_eq!(
        aggregation.entity, 1,
        "the diagnostic must name the entity whose decisions collapsed"
    );
    assert_eq!(aggregation.total_weight, 2);
}
