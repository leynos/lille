//! Tests for the vector math utility functions.
//! Focuses on normalisation and NaN handling.
use approx::assert_relative_eq;
use lille::vec_normalize;
use rstest::rstest;

const TOLERANCE: f32 = 1e-6;

#[rstest]
#[case::nan_x(f32::NAN, 1.0, 0.0, (0.0, 0.0, 0.0))]
#[case::unit_x(3.0, 0.0, 0.0, (1.0, 0.0, 0.0))]
#[case::zero(0.0, 0.0, 0.0, (0.0, 0.0, 0.0))]
#[case::infinite_x(f32::INFINITY, 0.0, 0.0, (0.0, 0.0, 0.0))]
#[case::neg_x_axis(-3.0, 0.0, 0.0, (-1.0, 0.0, 0.0))]
#[case::diagonal(1.0, 2.0, 2.0, (1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0))]
fn vec_normalize_returns_expected(
    #[case] x: f32,
    #[case] y: f32,
    #[case] z: f32,
    #[case] expected: (f32, f32, f32),
) {
    let (rx, ry, rz) = vec_normalize(x, y, z);
    let (ex, ey, ez) = expected;
    assert_relative_eq!(rx, ex, epsilon = TOLERANCE, max_relative = TOLERANCE);
    assert_relative_eq!(ry, ey, epsilon = TOLERANCE, max_relative = TOLERANCE);
    assert_relative_eq!(rz, ez, epsilon = TOLERANCE, max_relative = TOLERANCE);
}

#[test]
fn vec_mag_matches_pythagoras() {
    let mag = lille::vec_mag(3.0, 4.0, 12.0);
    assert_relative_eq!(mag, 13.0, max_relative = 1e-6);
}
