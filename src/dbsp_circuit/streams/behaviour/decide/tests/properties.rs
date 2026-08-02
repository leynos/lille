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
/// process. Single-entity, like the matrices: the per-entity fan-out is
/// pinned deterministically by `dedupe_separates_entities` instead, which
/// keeps this oracle a single running total.
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
            acc.apply(&decision(1, dx, dy), weight);
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
                "direction must be normalized or the zero vector, got ({dx}, {dy})"
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
        let outputs = deduped_outputs(&for_entity_one(&weighted))
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
