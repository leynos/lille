//! Unit tests for physics calculations.
//! Covers acceleration helper functions for edge cases and typical inputs.
use approx::assert_relative_eq;
use lille::applied_acceleration;
use rstest::rstest;

#[rstest]
#[case::explicit_mass((7.0, -14.0, 21.0), Some(7.0), Some((1.0, -2.0, 3.0)))]
#[case::default_mass((70.0, 0.0, 0.0), None, Some((1.0, 0.0, 0.0)))]
#[case::invalid_mass((1.0, 1.0, 1.0), Some(0.0), None)]
#[case::negative_mass((1.0, 1.0, 1.0), Some(-5.0), None)]
fn acceleration_cases(
    #[case] force: (f64, f64, f64),
    #[case] mass: Option<f64>,
    #[case] expected: Option<(f64, f64, f64)>,
) {
    let acc = applied_acceleration(force, mass);
    match (acc, expected) {
        (Some(a), Some(e)) => {
            assert_relative_eq!(a.0, e.0);
            assert_relative_eq!(a.1, e.1);
            assert_relative_eq!(a.2, e.2);
        }
        (None, None) => {}
        (a, e) => panic!("mismatch: {a:?} vs {e:?}"),
    }
}
