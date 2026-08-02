//! Direct unit tests for the pure helpers behind the DBSP output system.
//!
//! [`f32_from_f64`] and [`clamped_health_value`] neither log nor mutate, so
//! they are exercised here without building a world. The surrounding
//! orchestration — which turns their results into component writes and debug
//! logs — is covered by the behavioural tests in [`super::edge_cases`] and
//! [`super`].

use super::*;

#[rstest]
#[case::nan(f64::NAN, None)]
#[case::positive_infinity(f64::INFINITY, None)]
#[case::negative_infinity(f64::NEG_INFINITY, None)]
#[case::above_f32_range(1e300, None)]
#[case::below_f32_range(-1e300, None)]
#[case::in_range(1.5, Some(1.5))]
fn f32_from_f64_bounds(#[case] value: f64, #[case] expected: Option<f32>) {
    assert_eq!(f32_from_f64(value), expected);
}

/// Direct unit tests for the pure clamp helper. `clamped_health_value` does no
/// logging and no mutation, so it can be called without a world: it returns the
/// new value plus the pre-clamp total whenever clamping occurred, and
/// `apply_one_health_delta` is what turns that into a debug log.
///
/// The cases cover both bounds (`0` and `Health::max`) and the unclamped
/// interior, in both directions.
///
/// One case is one `(current, max, delta) -> (value, clamped_from)` row,
/// bundled into a struct so the generated test function stays within Clippy's
/// argument budget.
#[derive(Debug)]
struct ClampCase {
    current: u16,
    max: u16,
    delta: i32,
    expected_value: u16,
    expected_clamped_from: Option<i64>,
}

#[rstest]
#[case::unclamped_damage(ClampCase {
    current: 90, max: 100, delta: -50, expected_value: 40, expected_clamped_from: None,
})]
#[case::unclamped_heal(ClampCase {
    current: 40, max: 100, delta: 50, expected_value: 90, expected_clamped_from: None,
})]
#[case::exactly_zero_is_not_clamped(ClampCase {
    current: 50, max: 100, delta: -50, expected_value: 0, expected_clamped_from: None,
})]
#[case::exactly_max_is_not_clamped(ClampCase {
    current: 50, max: 100, delta: 50, expected_value: 100, expected_clamped_from: None,
})]
#[case::overkill_clamps_to_zero(ClampCase {
    current: 10, max: 100, delta: -50, expected_value: 0, expected_clamped_from: Some(-40),
})]
#[case::overheal_clamps_to_max(ClampCase {
    current: 90, max: 100, delta: 50, expected_value: 100, expected_clamped_from: Some(140),
})]
// Regression: `current + delta` exceeds `i32::MAX`, so computing the raw total
// in `i32` panicked in debug builds and wrapped to a negative value in release
// builds — which the clamp then silently accepted as 0. The raw total is
// computed in `i64`, and `clamped_from` reports it in full.
#[case::delta_overflows_i32(ClampCase {
    current: 90,
    max: 100,
    delta: i32::MAX,
    expected_value: 100,
    expected_clamped_from: Some(90 + i64::from(i32::MAX)),
})]
fn clamped_health_value_reports_value_and_clamping(#[case] case: ClampCase) {
    let health = Health {
        current: case.current,
        max: case.max,
    };
    let delta = HealthDelta {
        entity: 1,
        delta: case.delta,
        at_tick: 1,
        seq: Some(1),
        death: false,
    };

    let ClampedHealth {
        value,
        clamped_from,
    } = clamped_health_value(&delta, &health).expect("a clamped value always fits in u16");

    assert_eq!(
        value, case.expected_value,
        "the value must be the raw total clamped to 0..=max"
    );
    assert_eq!(
        clamped_from, case.expected_clamped_from,
        "clamped_from must carry the pre-clamp total exactly when clamping occurred"
    );
}
