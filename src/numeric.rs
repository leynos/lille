//! Numeric conversion helpers used across the project.
//!
//! These utilities guard conversions between floating-point and integer
//! domains. They rely on debug assertions to flag unexpected overflows while
//! keeping the call-sites ergonomic.

use ordered_float::OrderedFloat;

/// Convert a finite `f64` into `f32`, asserting that it fits the target type.
///
/// # Examples
/// ```rust
/// use lille::numeric::expect_f32;
/// let value = expect_f32(123.5_f64);
/// assert_eq!(value, 123.5_f32);
/// ```
///
/// ```rust
/// use lille::numeric::expect_f32;
/// // Debug builds panic because the value exceeds f32::MAX.
/// // expect_f32(f64::from(f32::MAX) + 1.0);
/// ```
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
///
/// # Examples
/// ```rust
/// use lille::numeric::expect_u16;
/// assert_eq!(expect_u16(42.0_f64), 42);
/// assert_eq!(expect_u16(f64::from(u16::MIN)), 0);
/// assert_eq!(expect_u16(f64::from(u16::MAX)), u16::MAX);
/// ```
///
/// Debug assertions trigger in debug builds if the value is non-finite or
/// outside the `u16` range; callers should clamp inputs before release builds.
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
///
/// # Examples
/// ```rust
/// use lille::numeric::floor_to_u16;
/// assert_eq!(floor_to_u16(123.9_f64), Some(123));
/// assert_eq!(floor_to_u16(70000.0_f64), None);
/// assert_eq!(floor_to_u16(f64::NAN), None);
/// ```
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

/// Floors an `OrderedFloat<f64>` into the `i32` domain.
///
/// Non-finite values (NaN or ±∞) yield `0`. Finite inputs are floored and then
/// clamped to `i32::MIN..=i32::MAX` before casting.
///
/// # Examples
/// ```rust
/// use ordered_float::OrderedFloat;
/// use lille::numeric::floor_to_i32;
///
/// assert_eq!(floor_to_i32(OrderedFloat(1.9_f64)), 1);
/// assert_eq!(floor_to_i32(OrderedFloat(-1.1_f64)), -2);
/// assert_eq!(floor_to_i32(OrderedFloat(f64::from(i32::MAX) + 10.0)), i32::MAX);
/// assert_eq!(floor_to_i32(OrderedFloat(f64::from(i32::MIN) - 10.0)), i32::MIN);
/// assert_eq!(floor_to_i32(OrderedFloat(f64::NAN)), 0);
/// assert_eq!(floor_to_i32(OrderedFloat(f64::INFINITY)), 0);
/// assert_eq!(floor_to_i32(OrderedFloat(f64::NEG_INFINITY)), 0);
/// ```
#[expect(
    clippy::cast_possible_truncation,
    reason = "The value is clamped to the i32 bounds before casting."
)]
#[must_use]
pub fn floor_to_i32(value: OrderedFloat<f64>) -> i32 {
    let raw = value.into_inner();
    if !raw.is_finite() {
        return 0;
    }
    let floored = raw.floor();
    let clamped = floored.clamp(f64::from(i32::MIN), f64::from(i32::MAX));
    clamped as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expect_f32_converts_in_range_values() {
        let value = 123.5_f64;

        let result = expect_f32(value);

        assert!((result - 123.5_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn expect_u16_converts_in_range_values() {
        let value = 512.0_f64;

        let result = expect_u16(value);

        assert_eq!(result, 512_u16);
    }

    #[test]
    fn floor_to_u16_returns_none_for_negative_values() {
        assert_eq!(floor_to_u16(-1.0_f64), None);
    }

    #[test]
    fn floor_to_u16_returns_none_for_overflow_values() {
        assert_eq!(floor_to_u16(f64::from(u16::MAX) + 1.0), None);
    }

    #[test]
    fn floor_to_u16_returns_none_for_non_finite_values() {
        assert_eq!(floor_to_u16(f64::INFINITY), None);
        assert_eq!(floor_to_u16(f64::NEG_INFINITY), None);
        assert_eq!(floor_to_u16(f64::NAN), None);
    }

    #[test]
    fn floor_to_u16_floors_valid_values() {
        assert_eq!(floor_to_u16(123.9_f64), Some(123_u16));
    }

    #[test]
    fn floor_to_i32_clamps_to_bounds() {
        let min = floor_to_i32(OrderedFloat::from(f64::from(i32::MIN) - 10.0));
        let max = floor_to_i32(OrderedFloat::from(f64::from(i32::MAX) + 10.0));

        assert_eq!(min, i32::MIN);
        assert_eq!(max, i32::MAX);
    }
}
