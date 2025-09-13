//! Compile-time guard ensuring `FearLevel` does not implement `Copy`.
use lille::dbsp_circuit::FearLevel;
use static_assertions::assert_not_impl_any;

#[test]
fn fear_level_is_not_copy() {
    assert_not_impl_any!(FearLevel: Copy);
}
