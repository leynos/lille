//! Utility helpers for tests.
//! Provides assertions for verifying generated code and constructors for
//! common physics records.

use dbsp::dynamic::{DynData, Erase};
use dbsp::typed_batch::OrdZSet;
use dbsp::{DBData, OutputHandle, ZWeight};

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

/// Consolidates a DBSP output handle into `(record, weight)` pairs.
///
/// Retaining the Z-set weight lets tests assert multiplicity — for example,
/// that a deduplicated stream emits a record exactly once — rather than only
/// checking the record's contents.
///
/// # Examples
/// ```rust
/// use dbsp::RootCircuit;
/// use test_utils::collect_weighted;
/// # fn main() -> Result<(), dbsp::Error> {
/// let (circuit, (input, output)) = RootCircuit::build(|circuit| {
///     let (stream, handle) = circuit.add_input_zset::<i64>();
///     Ok((handle, stream.output()))
/// })?;
///
/// // Pushing the same record twice consolidates to one record of weight 2.
/// input.push(7, 1);
/// input.push(7, 1);
/// circuit.step()?;
///
/// assert_eq!(collect_weighted(&output), vec![(7, 2)]);
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn collect_weighted<T>(handle: &OutputHandle<OrdZSet<T>>) -> Vec<(T, ZWeight)>
where
    T: DBData + Erase<DynData>,
{
    handle
        .consolidate()
        .iter()
        .map(|(record, (), weight)| (record, weight))
        .collect()
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
/// # fn main() -> Result<(), dbsp::Error> {
/// let mut circuit = new_circuit()?;
/// step(&mut circuit);
/// # Ok(())
/// # }
/// ```
pub use lille::dbsp_circuit::step;

/// Advances the circuit and includes context in panic messages.
///
/// # Examples
/// ```rust
/// use test_utils::{new_circuit, step_named};
/// # fn main() -> Result<(), dbsp::Error> {
/// let mut circuit = new_circuit()?;
/// step_named(&mut circuit, "ctx");
/// # Ok(())
/// # }
/// ```
pub use lille::dbsp_circuit::step_named;

#[cfg(test)]
mod tests {
    //! Tests for the shared assertion helpers.
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn expect_single_returns_sole_item() {
        let values = [7];
        assert_eq!(*expect_single(&values, "sole item"), 7);
    }

    #[rstest]
    #[should_panic(expected = "expected one item, found none")]
    fn expect_single_panics_on_empty_slice() {
        let _ = expect_single::<i32>(&[], "empty slice");
    }

    #[rstest]
    #[should_panic(expected = "expected one item, found 2")]
    fn expect_single_panics_on_multiple_items() {
        let _ = expect_single(&[1, 2], "multiple items");
    }

    #[rstest]
    fn assert_all_present_accepts_contained_keys() {
        assert_all_present("fn main() {}", &["fn", "main"]);
    }

    #[rstest]
    #[should_panic(expected = "missing not found in output")]
    fn assert_all_present_panics_on_missing_key() {
        assert_all_present("fn main() {}", &["missing"]);
    }

    #[rstest]
    fn assert_all_absent_accepts_absent_keys() {
        assert_all_absent("fn main() {}", &["struct", "enum"]);
    }

    #[rstest]
    #[should_panic(expected = "fn should not be present")]
    fn assert_all_absent_panics_on_present_key() {
        assert_all_absent("fn main() {}", &["fn"]);
    }

    #[rstest]
    fn assert_valid_rust_syntax_accepts_valid_code() {
        assert_valid_rust_syntax("fn main() {}");
    }

    #[rstest]
    #[should_panic(expected = "generated code is not valid Rust")]
    fn assert_valid_rust_syntax_panics_on_invalid_code() {
        assert_valid_rust_syntax("fn main( {");
    }
}
