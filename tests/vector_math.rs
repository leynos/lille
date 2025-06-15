use approx::assert_relative_eq;
use lille::vec_normalize;

#[test]
fn normalize_returns_zero_for_nan() {
    let result = vec_normalize(f32::NAN, 1.0, 0.0);
    assert_relative_eq!(result.0, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.1, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.2, 0.0, max_relative = 1e-6);
}

#[test]
fn normalize_returns_normalized_vector() {
    let result = vec_normalize(3.0, 0.0, 0.0);
    assert_relative_eq!(result.0, 1.0, max_relative = 1e-6);
    assert_relative_eq!(result.1, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.2, 0.0, max_relative = 1e-6);
}

#[test]
fn normalize_returns_zero_for_zero_vector() {
    let result = vec_normalize(0.0, 0.0, 0.0);
    assert_relative_eq!(result.0, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.1, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.2, 0.0, max_relative = 1e-6);
}

#[test]
fn normalize_returns_zero_for_infinite_vector() {
    let result = vec_normalize(f32::INFINITY, 0.0, 0.0);
    assert_relative_eq!(result.0, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.1, 0.0, max_relative = 1e-6);
    assert_relative_eq!(result.2, 0.0, max_relative = 1e-6);
}
