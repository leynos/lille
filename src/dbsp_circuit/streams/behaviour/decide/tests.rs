//! Unit tests for the private movement accumulator and dedupe stream.
use super::*;
use approx::assert_relative_eq;
use rstest::rstest;

fn decision(dx: f64, dy: f64) -> MovementDecision {
    MovementDecision {
        entity: 1,
        dx: OrderedFloat(dx),
        dy: OrderedFloat(dy),
    }
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
fn deduped_outputs(weighted: &[((f64, f64), i64)]) -> Result<DedupeOutputs, dbsp::Error> {
    let (circuit, (input, decisions_out, aggregations_out)) = RootCircuit::build(|circuit| {
        let (stream, handle) = circuit.add_input_zset::<MovementDecision>();
        let (deduped, aggregations) = dedupe_movement_decisions(&stream);
        Ok((handle, deduped.output(), aggregations.output()))
    })?;
    for &((dx, dy), weight) in weighted {
        input.push(decision(dx, dy), weight);
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
    Ok(deduped_outputs(weighted)?.decisions)
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
        acc.apply(&decision(dx, dy), weight);
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
    let outputs = deduped_outputs(weighted).expect("dedupe circuit run");
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

mod properties {
    //! Property-based coverage supplementing the bounded matrices above.
    //!
    //! The matrices pin a handful of representative shapes exactly; these
    //! properties sample the broader domain of weighted decision sets, including
    //! extreme `i64` weights, and check the invariants that must hold across all
    //! of it. See `docs/adr-003-bounded-rstest-over-property-testing.md`.

    use super::*;
    use proptest::prelude::*;

    /// Unit-ish directions plus the zero vector: the domain
    /// `decide_movement` actually produces. Generating unbounded floats would
    /// only exercise `f64` overflow, which is not what deduplication is
    /// responsible for.
    fn direction_strategy() -> impl Strategy<Value = (f64, f64)> {
        prop_oneof![
            2 => (0.0f64..std::f64::consts::TAU).prop_map(|angle| (angle.cos(), angle.sin())),
            1 => Just((0.0, 0.0)),
        ]
    }

    /// Z-set weights spanning the ordinary range and both `i64` extremes. The
    /// extremes are included deliberately: `i64::MIN` is the value whose
    /// `abs()` overflows, and summing extremes is what forced the accumulator's
    /// `saturating_add`.
    fn extreme_weight_strategy() -> impl Strategy<Value = i64> {
        prop_oneof![
            8 => -4i64..=4,
            1 => Just(i64::MIN),
            1 => Just(i64::MAX),
        ]
    }

    /// Weights for the circuit-level property. Bounded, because DBSP's own
    /// `ZWeight` arithmetic panics with `attempt to add with overflow` on
    /// extreme weights before any of this crate's code runs: those values are
    /// outside what the circuit can process at all, so a property asserting
    /// deduplication behaviour cannot use them. The accumulator's own handling
    /// of the extremes is covered by the pure property above, which does not go
    /// through DBSP.
    fn zset_weight_strategy() -> impl Strategy<Value = i64> {
        -4i64..=4
    }

    /// Overflow-safe oracle for the accumulated total.
    ///
    /// Folds in `i128` and clamps after each addition, mirroring the
    /// accumulator's per-step `saturating_add`. Summing first and clamping once
    /// would not match: saturation is order-dependent, so `[MAX, MAX, MIN]`
    /// saturates to `-1` step by step but sums to `MAX - 1`.
    /// Returned as `i128` so the oracle never needs a fallible narrowing
    /// conversion; callers widen the accumulator's `i64` total to compare.
    fn expected_total_weight(weighted: &[((f64, f64), i64)]) -> i128 {
        let (min, max) = (i128::from(i64::MIN), i128::from(i64::MAX));
        weighted.iter().fold(0i128, |total, &(_, weight)| {
            (total + i128::from(weight)).clamp(min, max)
        })
    }

    fn weighted_decisions_strategy() -> impl Strategy<Value = Vec<((f64, f64), i64)>> {
        prop::collection::vec((direction_strategy(), extreme_weight_strategy()), 1..8)
    }

    /// As [`weighted_decisions_strategy`], with weights DBSP can actually
    /// process.
    fn zset_decisions_strategy() -> impl Strategy<Value = Vec<((f64, f64), i64)>> {
        prop::collection::vec((direction_strategy(), zset_weight_strategy()), 1..8)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// `to_decision` never panics, and its decision is present exactly when
        /// the overflow-safe total is non-zero. The emitted direction is always
        /// finite, and is either a unit vector or the defined zero vector when
        /// the averaged magnitude falls below `MIN_DIRECTION_MAGNITUDE`.
        #[test]
        fn to_decision_matches_the_overflow_safe_weight_oracle(
            weighted in weighted_decisions_strategy(),
        ) {
            let mut acc = MovementAccumulator::default();
            for &((dx, dy), weight) in &weighted {
                acc.apply(&decision(dx, dy), weight);
            }
            let expected_total = expected_total_weight(&weighted);
            prop_assert_eq!(i128::from(acc.total_weight), expected_total);

            let outcome = acc.to_decision(1);
            prop_assert_eq!(outcome.decision.is_some(), expected_total != 0);

            if let Some(movement) = outcome.decision {
                let dx = movement.dx.into_inner();
                let dy = movement.dy.into_inner();
                prop_assert!(dx.is_finite() && dy.is_finite(), "direction must be finite");
                let magnitude = dx.hypot(dy);
                prop_assert!(
                    (magnitude - 1.0).abs() < 1e-9 || (dx == 0.0 && dy == 0.0),
                    "direction must be normalised or the zero vector, got ({dx}, {dy})"
                );
            }

            // The diagnostic is reported exactly when more than one decision
            // folded in, tested against the same overflow-safe total.
            prop_assert_eq!(
                outcome.aggregation.is_some(),
                !(-1i128..=1).contains(&expected_total)
            );
        }
    }

    proptest! {
        // Each case builds and steps a circuit, so this runs fewer cases than
        // the pure property above.
        #![proptest_config(ProptestConfig::with_cases(48))]

        /// End to end through `dedupe_movement_decisions`: a net-zero total
        /// emits nothing, and any non-zero total emits exactly one consolidated
        /// decision with Z-set multiplicity 1.
        #[test]
        fn dedupe_emits_at_most_one_decision_of_multiplicity_one(
            weighted in zset_decisions_strategy(),
        ) {
            let outputs = deduped_outputs(&weighted)
                .map_err(|error| TestCaseError::fail(format!("circuit run failed: {error}")))?;
            let expected_total = expected_total_weight(&weighted);

            if expected_total == 0 {
                prop_assert!(
                    outputs.decisions.is_empty(),
                    "a net-zero total must emit no decision, got {:?}",
                    outputs.decisions
                );
            } else {
                let (_, weight) = test_utils::expect_single(
                    &outputs.decisions,
                    "a non-zero total must emit exactly one decision",
                );
                prop_assert_eq!(*weight, 1, "the deduplicated decision must have multiplicity 1");
            }
        }
    }
}
