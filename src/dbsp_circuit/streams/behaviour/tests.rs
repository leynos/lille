//! Tests for the behavioural movement streams.

use super::decide::{decide_movement, PositionTarget};
use super::{fear_level_stream, movement_decision_stream};
use crate::dbsp_circuit::{FearLevel, MovementDecision, Position, Target};
use crate::FEAR_THRESHOLD;
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
    let (circuit, (fear_in, target_in, pos_in, decisions_handle)) =
        build_decision_circuit().expect("failed to build circuit for movement_decision_join");

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
        .map(|(decision, (), _timestamp)| {
            let decision_ref: &MovementDecision = &decision;
            *decision_ref
        })
        .collect();
    let decision = test_utils::expect_single(&decisions, "movement decision result");
    assert_relative_eq!(decision.dx.into_inner(), expected_dx);
    assert_relative_eq!(decision.dy.into_inner(), expected_dy);
}

#[test]
fn no_decision_without_target() {
    // The target handle is intentionally left unused: this test pushes no
    // target, so no movement decision should be produced.
    let (circuit, (fear_in, _target_in, pos_in, decisions_handle)) =
        build_decision_circuit().expect("failed to build circuit for no_decision_without_target");

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
        .map(|(decision, (), _timestamp)| {
            let decision_ref: &MovementDecision = &decision;
            *decision_ref
        })
        .collect();
    assert!(decisions.is_empty());
}

#[rstest]
fn decide_movement_zero_displacement_yields_zero_vector() {
    let mv = decide_movement(0.0.into(), &pt(1.0, 1.0, 1.0, 1.0));
    assert_relative_eq!(mv.dx.into_inner(), 0.0);
    assert_relative_eq!(mv.dy.into_inner(), 0.0);
}

/// Builds the movement-decision circuit used by the multi-target tests.
///
/// Construction can fail, so the error propagates to the calling test rather
/// than panicking here.
#[expect(
    clippy::type_complexity,
    reason = "DBSP handle tuples are verbose by nature"
)]
fn build_decision_circuit() -> Result<
    (
        dbsp::CircuitHandle,
        (
            dbsp::ZSetHandle<FearLevel>,
            dbsp::ZSetHandle<Target>,
            dbsp::ZSetHandle<Position>,
            dbsp::OutputHandle<dbsp::typed_batch::OrdZSet<MovementDecision>>,
        ),
    ),
    dbsp::Error,
> {
    let handles = RootCircuit::build(|circuit| {
        let (fear_input, fear_handle) = circuit.add_input_zset::<FearLevel>();
        let (target_stream, target_handle) = circuit.add_input_zset::<Target>();
        let (position_stream, position_handle) = circuit.add_input_zset::<Position>();
        let fear_stream = fear_level_stream(&position_stream, &fear_input);
        let output_handle =
            movement_decision_stream(&fear_stream, &target_stream, &position_stream).output();
        Ok((fear_handle, target_handle, position_handle, output_handle))
    })?;
    Ok(handles)
}

fn collect_decisions(
    handle: &dbsp::OutputHandle<dbsp::typed_batch::OrdZSet<MovementDecision>>,
) -> Vec<MovementDecision> {
    handle
        .consolidate()
        .iter()
        .map(|(decision, (), _timestamp)| {
            let decision_ref: &MovementDecision = &decision;
            *decision_ref
        })
        .collect()
}

#[rstest]
#[case::distinct_targets_average(
    (2.0, 0.0),
    (0.0, 2.0),
    Some((std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2))
)]
#[case::opposing_targets_cancel((2.0, 0.0), (-2.0, 0.0), Some((0.0, 0.0)))]
fn conflicting_targets_normalise_to_one_decision(
    #[case] first_target: (f64, f64),
    #[case] second_target: (f64, f64),
    #[case] expected: Option<(f64, f64)>,
) {
    let (circuit, (fear_in, target_in, pos_in, decisions_handle)) =
        build_decision_circuit().expect("failed to build circuit for conflicting target test");

    fear_in.push(
        FearLevel {
            entity: 1,
            level: 0.0.into(),
        },
        1,
    );
    for (tx, ty) in [first_target, second_target] {
        target_in.push(
            Target {
                entity: 1,
                x: tx.into(),
                y: ty.into(),
            },
            1,
        );
    }
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

    let decisions = collect_decisions(&decisions_handle);
    match expected {
        Some((expected_dx, expected_dy)) => {
            let decision = test_utils::expect_single(&decisions, "conflicting target decision");
            assert_relative_eq!(decision.dx.into_inner(), expected_dx);
            assert_relative_eq!(decision.dy.into_inner(), expected_dy);
        }
        None => assert!(decisions.is_empty()),
    }
}

#[test]
fn duplicate_targets_produce_single_decision() {
    let (circuit, (fear_in, target_in, pos_in, decisions_handle)) =
        build_decision_circuit().expect("failed to build circuit for duplicate target test");

    fear_in.push(
        FearLevel {
            entity: 1,
            level: 0.0.into(),
        },
        1,
    );

    let target = Target {
        entity: 1,
        x: 5.0.into(),
        y: (-3.0).into(),
    };
    target_in.push(target, 1);
    target_in.push(target, 1);

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
        .map(|(decision, (), _timestamp)| {
            let decision_ref: &MovementDecision = &decision;
            *decision_ref
        })
        .collect();
    let decision = test_utils::expect_single(&decisions, "movement decision result");
    let magnitude = (5_f64.powi(2) + (-3_f64).powi(2)).sqrt();
    assert_relative_eq!(decision.dx.into_inner(), 5.0 / magnitude);
    assert_relative_eq!(decision.dy.into_inner(), -3.0 / magnitude);
}
