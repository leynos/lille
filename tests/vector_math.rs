//! Tests for the vector math utility functions.
//! Focuses on normalisation and NaN handling.
use approx::assert_relative_eq;
use lille::vec_normalize;
use rstest::rstest;

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
    let result = vec_normalize(x, y, z);
    assert_relative_eq!(result.0, expected.0, epsilon = 1e-6, max_relative = 1e-6);
    assert_relative_eq!(result.1, expected.1, epsilon = 1e-6, max_relative = 1e-6);
    assert_relative_eq!(result.2, expected.2, epsilon = 1e-6, max_relative = 1e-6);
}

#[test]
fn vec_mag_matches_pythagoras() {
    let mag = lille::vec_mag(3.0, 4.0, 12.0);
    assert_relative_eq!(mag, 13.0, max_relative = 1e-6);
}
