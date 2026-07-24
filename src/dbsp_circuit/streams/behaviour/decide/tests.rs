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

/// Runs a single entity's weighted decisions through
/// [`dedupe_movement_decisions`] and returns each emitted decision paired
/// with its consolidated Z-set weight (multiplicity).
///
/// Returns the fallible `Result` (rather than unwrapping) so the circuit
/// construction stays outside a `no_expect_outside_tests` boundary; callers
/// unwrap it.
fn deduped_decisions(
    weighted: &[((f64, f64), i64)],
) -> Result<Vec<(MovementDecision, i64)>, dbsp::Error> {
    let (circuit, (input, output)) = RootCircuit::build(|circuit| {
        let (stream, handle) = circuit.add_input_zset::<MovementDecision>();
        let deduped = dedupe_movement_decisions(&stream).output();
        Ok((handle, deduped))
    })?;
    for &((dx, dy), weight) in weighted {
        input.push(decision(dx, dy), weight);
    }
    circuit.step()?;
    Ok(output
        .consolidate()
        .iter()
        .map(|(movement, (), weight)| {
            let movement_ref: &MovementDecision = &movement;
            (*movement_ref, weight)
        })
        .collect())
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
        .expect("net weight of one must yield a decision");
    assert_relative_eq!(movement.dx.into_inner(), 1.0);
    assert_relative_eq!(movement.dy.into_inner(), 0.0);
}
