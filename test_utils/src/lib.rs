//! Utility helpers for tests.
//! Provides assertions for verifying generated code and constructors for
//! common physics records.

pub mod conversions;
pub mod dbsp_sync;
pub mod physics;
pub use physics::{
    block, fear, force, force_with_mass, new_circuit, pos, slope, target, vel, BlockCoords,
    BlockId, Coords2D, Coords3D, EntityId, FearValue, ForceVector, Gradient, Mass,
};

pub mod prelude {
    //! Re-export commonly used test helpers.
    pub use super::{
        assert_all_absent, assert_all_present, assert_valid_rust_syntax, block,
        dbsp_sync::install_error_observer, dbsp_sync::CapturedErrors, expect_single, fear, force,
        force_with_mass, new_circuit, pos, slope, target, vel, BlockCoords, BlockId, Coords2D,
        Coords3D, EntityId, FearValue, ForceVector, Gradient, Mass,
    };
}

/// Extract a single item from a slice, panicking with a contextual message when
/// the slice is empty or contains multiple entries.
///
/// # Examples
/// ```rust
/// use test_utils::expect_single;
/// let values = [42];
/// let item = expect_single(&values, "single value");
/// assert_eq!(*item, 42);
/// ```
///
/// ```rust,should_panic
/// use test_utils::expect_single;
/// expect_single::<i32>(&[], "empty slice");
/// ```
///
/// ```rust,should_panic
/// use test_utils::expect_single;
/// expect_single(&[1, 2], "multiple items");
/// ```
#[must_use]
pub fn expect_single<'a, T>(items: &'a [T], context: &str) -> &'a T {
    match items {
        [item] => item,
        [] => panic!("{context}: expected one item, found none"),
        many => panic!("{context}: expected one item, found {}", many.len()),
    }
}

/// Assert that all strings in `keys` are present in `code`.
///
/// # Panics
/// Panics with a helpful message if any key is missing.
pub fn assert_all_present(code: &str, keys: &[&str]) {
    for key in keys {
        assert!(code.contains(key), "{key} not found in output");
    }
}

/// Assert that all strings in `keys` are absent from `code`.
///
/// # Panics
/// Panics with a helpful message if any key is found.
pub fn assert_all_absent(code: &str, keys: &[&str]) {
    for key in keys {
        assert!(!code.contains(key), "{key} should not be present");
    }
}

/// Basic sanity checks that generated code is syntactically valid Rust.
pub fn assert_valid_rust_syntax(code: &str) {
    use syn::parse_file; // syn = syntax only, no type-checking

    if let Err(err) = parse_file(code) {
        panic!("generated code is not valid Rust:\n{code}\nError: {err}",);
    }
}

/// Step the circuit and panic if evaluation fails.
///
/// # Examples
/// ```rust
/// use test_utils::{new_circuit, step};
/// let mut circuit = new_circuit();
/// step(&mut circuit);
/// ```
pub use lille::dbsp_circuit::step;

/// Advances the circuit and includes context in panic messages.
///
/// # Examples
/// ```rust
/// use test_utils::{new_circuit, step_named};
/// let mut circuit = new_circuit();
/// step_named(&mut circuit, "ctx");
/// ```
pub use lille::dbsp_circuit::step_named;
