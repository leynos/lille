use lille::vec_normalize;

#[test]
fn normalize_returns_zero_for_nan() {
    let result = vec_normalize(f32::NAN, 1.0, 0.0);
    assert_eq!(result, (0.0, 0.0, 0.0));
}

#[test]
fn normalize_returns_normalized_vector() {
    let result = vec_normalize(3.0, 0.0, 0.0);
    assert_eq!(result, (1.0, 0.0, 0.0));
}
