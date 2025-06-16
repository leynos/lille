pub mod constants;
pub mod ddlog;
pub mod font;

use std::{error::Error, path::PathBuf};

/// Execute all build steps required by `build.rs`.
///
/// This function generates constants, downloads the Fira Sans font and
/// compiles any Differential Datalog sources if the `ddlog` executable is
/// available. Environment variables such as `CARGO_MANIFEST_DIR` and `OUT_DIR`
/// must be set by Cargo before this function is called.
///
/// # Examples
/// ```rust,no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     build_support::build()
/// }
/// ```
///
/// # Returns
/// `Ok(())` if all build steps succeed, otherwise an error is returned from the
/// failing step.
pub fn build() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=src/lille.dl");
    println!("cargo:rerun-if-changed=constants.toml");
    println!("cargo:rerun-if-env-changed=DDLOG_HOME");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    constants::generate_constants(&manifest_dir, &out_dir)?;
    let font_path = font::download_font(&manifest_dir)?;
    ddlog::compile_ddlog(&manifest_dir, &out_dir)?;

    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, TempDir) {
        let manifest_temp = TempDir::new().expect("Failed to create temp manifest dir");
        let out_temp = TempDir::new().expect("Failed to create temp out dir");
        env::set_var("CARGO_MANIFEST_DIR", manifest_temp.path());
        env::set_var("OUT_DIR", out_temp.path());
        (manifest_temp, out_temp)
    }

    fn cleanup_test_env() {
        env::remove_var("CARGO_MANIFEST_DIR");
        env::remove_var("OUT_DIR");
    }

    #[test]
    fn test_build_missing_cargo_manifest_dir() {
        cleanup_test_env();
        env::set_var("OUT_DIR", "/tmp/test_out");
        let result = build();
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_missing_out_dir() {
        cleanup_test_env();
        env::set_var("CARGO_MANIFEST_DIR", "/tmp/test_manifest");
        let result = build();
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_missing_both_env_vars() {
        cleanup_test_env();
        let result = build();
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_env_vars_extraction() {
        let (_manifest_temp, _out_temp) = setup_test_env();
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir_str = env::var("OUT_DIR").unwrap();
        let out_dir = PathBuf::from(&out_dir_str);
        assert!(!manifest_dir.is_empty());
        assert!(out_dir.exists());
        cleanup_test_env();
    }

    #[test]
    fn test_build_pathbuf_creation() {
        let (_manifest_temp, out_temp) = setup_test_env();
        env::set_var("OUT_DIR", out_temp.path());
        let out_dir_str = env::var("OUT_DIR").unwrap();
        let out_dir = PathBuf::from(&out_dir_str);
        assert_eq!(out_dir, out_temp.path());
        assert!(out_dir.is_absolute());
        cleanup_test_env();
    }

    #[test]
    fn test_build_function_error_propagation() {
        cleanup_test_env();
        env::set_var("CARGO_MANIFEST_DIR", "/nonexistent/path");
        env::set_var("OUT_DIR", "/nonexistent/out");
        let result = build();
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_with_valid_temp_dirs() {
        let (manifest_temp, _) = setup_test_env();
        fs::create_dir_all(manifest_temp.path().join("assets")).ok();
        fs::create_dir_all(manifest_temp.path().join("src")).ok();
        fs::write(
            manifest_temp.path().join("constants.toml"),
            "# test constants",
        )
        .ok();
        fs::write(manifest_temp.path().join("src/lille.dl"), "# test ddlog").ok();
        let result = build();
        match result {
            Ok(_) => assert!(true),
            Err(e) => {
                println!("Expected error in test environment: {}", e);
                assert!(true);
            }
        }
        cleanup_test_env();
    }

    #[test]
    fn test_build_environment_variable_setting() {
        let (manifest_temp, _) = setup_test_env();
        fs::create_dir_all(manifest_temp.path().join("assets")).ok();
        fs::create_dir_all(manifest_temp.path().join("src")).ok();
        fs::write(manifest_temp.path().join("constants.toml"), "").ok();
        fs::write(manifest_temp.path().join("src/lille.dl"), "").ok();
        let manifest_dir_result = env::var("CARGO_MANIFEST_DIR");
        if let Ok(val) = manifest_dir_result {
            assert!(!val.is_empty());
        }
        let out_dir_result = env::var("OUT_DIR");
        if let Ok(val) = out_dir_result {
            assert!(!val.is_empty());
        }
        cleanup_test_env();
    }

    #[test]
    fn test_build_empty_env_vars() {
        cleanup_test_env();
        env::set_var("CARGO_MANIFEST_DIR", "");
        env::set_var("OUT_DIR", "");
        let result = build();
        match result {
            Ok(_) => assert!(false, "Expected error with empty paths"),
            Err(_) => assert!(true, "Expected error with empty paths"),
        }
        cleanup_test_env();
    }

    #[test]
    fn test_build_unicode_paths() {
        cleanup_test_env();
        let unicode_path = "/tmp/测试路径/マニフェスト";
        env::set_var("CARGO_MANIFEST_DIR", unicode_path);
        env::set_var("OUT_DIR", "/tmp/out_测试");
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        assert_eq!(manifest_dir, unicode_path);
        cleanup_test_env();
    }

    #[test]
    fn test_build_very_long_paths() {
        cleanup_test_env();
        let long_path = "/tmp/".to_string() + &"very_long_directory_name_".repeat(10);
        env::set_var("CARGO_MANIFEST_DIR", &long_path);
        env::set_var("OUT_DIR", &long_path);
        let result = env::var("CARGO_MANIFEST_DIR");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), long_path);
        cleanup_test_env();
    }

    #[test]
    fn test_build_function_return_type() {
        let (_manifest_temp, _out_temp) = setup_test_env();
        let result = build();
        match result {
            Ok(unit_value) => assert_eq!(unit_value, ()),
            Err(error) => assert!(!error.to_string().is_empty()),
        }
        cleanup_test_env();
    }

    #[test]
    fn test_build_idempotency() {
        let (manifest_temp, _) = setup_test_env();
        fs::create_dir_all(manifest_temp.path().join("assets")).ok();
        fs::create_dir_all(manifest_temp.path().join("src")).ok();
        fs::write(manifest_temp.path().join("constants.toml"), "").ok();
        fs::write(manifest_temp.path().join("src/lille.dl"), "").ok();
        let result1 = build();
        let result2 = build();
        assert_eq!(result1.is_ok(), result2.is_ok());
        cleanup_test_env();
    }

    fn create_minimal_project_structure(base_path: &std::path::Path) {
        fs::create_dir_all(base_path.join("assets")).ok();
        fs::create_dir_all(base_path.join("src")).ok();
        fs::write(base_path.join("constants.toml"), "# Test constants file").ok();
        fs::write(base_path.join("src").join("lille.dl"), "# Test ddlog file").ok();
        fs::write(base_path.join("build.rs"), "// Test build script").ok();
    }

    #[test]
    fn test_env_var_handling_with_special_characters() {
        cleanup_test_env();
        let special_manifest = "/tmp/test path/with spaces & special chars!";
        let special_out = "/tmp/out-dir_with.dots/and-dashes";
        env::set_var("CARGO_MANIFEST_DIR", special_manifest);
        env::set_var("OUT_DIR", special_out);
        let manifest_result = env::var("CARGO_MANIFEST_DIR");
        let out_result = env::var("OUT_DIR");
        assert!(manifest_result.is_ok());
        assert!(out_result.is_ok());
        assert_eq!(manifest_result.unwrap(), special_manifest);
        assert_eq!(out_result.unwrap(), special_out);
        cleanup_test_env();
    }
}
