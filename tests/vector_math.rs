//! Tests for the vector math utility functions.
//! Focuses on normalisation and NaN handling.
use approx::assert_relative_eq;
use lille::vec_normalize;
use rstest::rstest;

#[rstest]
#[case(f32::NAN, 1.0, 0.0, (0.0, 0.0, 0.0))]
#[case(3.0, 0.0, 0.0, (1.0, 0.0, 0.0))]
#[case(0.0, 0.0, 0.0, (0.0, 0.0, 0.0))]
#[case(f32::INFINITY, 0.0, 0.0, (0.0, 0.0, 0.0))]
fn vec_normalize_returns_expected(
    #[case] x: f32,
    #[case] y: f32,
    #[case] z: f32,
    #[case] expected: (f32, f32, f32),
) {
    let result = vec_normalize(x, y, z);
    assert_relative_eq!(result.0, expected.0, max_relative = 1e-6);
    assert_relative_eq!(result.1, expected.1, max_relative = 1e-6);
    assert_relative_eq!(result.2, expected.2, max_relative = 1e-6);
}

#[test]
fn vec_mag_matches_pythagoras() {
    let mag = lille::vec_mag(3.0, 4.0, 12.0);
    assert_relative_eq!(mag, 13.0, max_relative = 1e-6);
}
