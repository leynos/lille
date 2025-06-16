/// Test helper utilities.

/// Assert that all strings in `keys` are present in `code`.
///
/// # Panics
/// Panics with a helpful message if any key is missing.
pub fn assert_all_present(code: &str, keys: &[&str]) {
    for key in keys {
        assert!(code.contains(key), "{} not found in output", key);
    }
}

/// Assert that all strings in `keys` are absent from `code`.
///
/// # Panics
/// Panics with a helpful message if any key is found.
pub fn assert_all_absent(code: &str, keys: &[&str]) {
    for key in keys {
        assert!(!code.contains(key), "{} should not be present", key);
    }
}

/// Basic sanity checks that generated code is syntactically valid Rust.
pub fn assert_valid_rust_syntax(code: &str) {
    use syn::parse_file; // syn = syntax only, no type-checking

    if let Err(err) = parse_file(code) {
        panic!(
            "generated code is not valid Rust:\n{}\nError: {}",
            code, err
        );
    }
}
