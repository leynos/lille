//! Numeric conversion helpers used across the project.
//!
//! These utilities guard conversions between floating-point and integer
//! domains. They rely on debug assertions to flag unexpected overflows while
//! keeping the call-sites ergonomic.

use ordered_float::OrderedFloat;

/// Convert a finite `f64` into `f32`, asserting that it fits the target type.
#[expect(
    clippy::cast_possible_truncation,
    reason = "Callers assert that the value fits within f32 bounds."
)]
#[must_use]
pub fn expect_f32(value: f64) -> f32 {
    debug_assert!(value.is_finite(), "expected finite f64 for f32 conversion");
    debug_assert!(
        value <= f64::from(f32::MAX),
        "f64 value {value} exceeds f32::MAX"
    );
    debug_assert!(
        value >= f64::from(f32::MIN),
        "f64 value {value} is below f32::MIN"
    );
    value as f32
}

/// Convert a finite `f64` into `u16`, asserting that it resides within bounds.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Callers clamp inputs before converting to u16."
)]
#[must_use]
pub fn expect_u16(value: f64) -> u16 {
    debug_assert!(value.is_finite(), "expected finite f64 for u16 conversion");
    debug_assert!(
        value >= f64::from(u16::MIN),
        "f64 value {value} is below u16::MIN"
    );
    debug_assert!(
        value <= f64::from(u16::MAX),
        "f64 value {value} exceeds u16::MAX"
    );
    value as u16
}

/// Floor the value and convert to `u16`, returning `None` when out of range.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "The floored value is validated against the u16 domain."
)]
#[must_use]
pub fn floor_to_u16(value: f64) -> Option<u16> {
    if !value.is_finite() {
        return None;
    }
    let floored = value.floor();
    if floored < f64::from(u16::MIN) || floored > f64::from(u16::MAX) {
        return None;
    }
    Some(floored as u16)
}

/// Floor an `OrderedFloat<f64>` and clamp it into the `i32` domain.
#[expect(
    clippy::cast_possible_truncation,
    reason = "The value is clamped to the i32 bounds before casting."
)]
#[must_use]
pub fn floor_to_i32(value: OrderedFloat<f64>) -> i32 {
    let floored = value.into_inner().floor();
    let clamped = floored.clamp(f64::from(i32::MIN), f64::from(i32::MAX));
    clamped as i32
}
