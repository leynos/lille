//! Unit tests for physics calculations.
use approx::assert_relative_eq;
use lille::DEFAULT_MASS;

fn applied_acceleration(force: (f32, f32, f32), mass: Option<f32>) -> Option<(f32, f32, f32)> {
    match mass {
        Some(m) if m > 0.0 => Some((force.0 / m, force.1 / m, force.2 / m)),
        Some(_) => None,
        None => Some((
            force.0 / DEFAULT_MASS as f32,
            force.1 / DEFAULT_MASS as f32,
            force.2 / DEFAULT_MASS as f32,
        )),
    }
}

#[test]
fn acceleration_divides_force_by_mass() {
    let acc = applied_acceleration((7.0, -14.0, 21.0), Some(7.0)).unwrap();
    assert_relative_eq!(acc.0, 1.0);
    assert_relative_eq!(acc.1, -2.0);
    assert_relative_eq!(acc.2, 3.0);
}

#[test]
fn acceleration_uses_default_mass_when_missing() {
    let acc = applied_acceleration((70.0, 0.0, 0.0), None).unwrap();
    assert_relative_eq!(acc.0, 1.0);
    assert_relative_eq!(acc.1, 0.0);
    assert_relative_eq!(acc.2, 0.0);
}

#[test]
fn acceleration_filters_non_positive_mass() {
    assert!(applied_acceleration((1.0, 1.0, 1.0), Some(0.0)).is_none());
    assert!(applied_acceleration((1.0, 1.0, 1.0), Some(-5.0)).is_none());
}
