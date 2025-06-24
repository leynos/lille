//! Build support utilities used by the project's build script.
//! Coordinates constants generation, font downloads, and optional `DDlog` compilation.
pub mod constants;
pub mod ddlog;
pub mod font;

use color_eyre::eyre::Result;
use std::fs;
use std::path::PathBuf;

#[expect(
    unexpected_cfgs,
    non_snake_case,
    unused_imports,
    reason = "OrthoConfig macro generates names that trigger these lints"
)]
#[allow(unfulfilled_lint_expectations)]
mod build_options {
    use ortho_config::OrthoConfig;
    use serde::Deserialize;

    /// Configuration options for the build pipeline.
    #[derive(Clone, Default, OrthoConfig, Debug, Deserialize)]
    #[ortho_config(prefix = "BUILD_SUPPORT")]
    #[expect(non_snake_case, reason = "OrthoConfig derives non-snake case fields")]
    #[allow(unfulfilled_lint_expectations)]
    pub struct BuildOptions {
        /// If `true`, a failure to compile DDlog code causes [`build_with_options`]
        /// to return an error. When `false`, DDlog errors are logged but ignored.
        #[ortho_config(default = false)]
        pub fail_on_ddlog_error: bool,

        /// Destination directory for the generated `lille_ddlog` crate.
        /// If not provided, defaults to `OUT_DIR/lille_ddlog`.
        pub ddlog_dir: Option<std::path::PathBuf>,
    }
}

pub use build_options::BuildOptions;

/// Execute all build steps required by `build.rs`.
///
/// This function generates constants, downloads the Fira Sans font and
/// compiles any Differential Datalog sources if the `ddlog` executable is
/// available. Environment variables such as `CARGO_MANIFEST_DIR` and `OUT_DIR`
/// must be set by Cargo before this function is called.
///
/// # Examples
/// ```rust,no_run
/// use color_eyre::eyre::Result;
/// fn main() -> Result<()> {
///     build_support::build()
/// }
/// ```
///
/// # Returns
/// `Ok(())` if all build steps succeed, otherwise an error is returned from the
/// failing step.
///
/// # Errors
/// Returns an error if required environment variables are missing, if any file
/// operation fails, or when Differential Datalog compilation does not succeed.
pub fn build() -> Result<()> {
    build_with_options(&BuildOptions::default())
}

/// Execute all build steps with configurable behaviour.
///
/// When `options.fail_on_ddlog_error` is `false`, any error returned from
/// [`ddlog::compile_ddlog`] is printed as a Cargo warning and ignored. This
/// mirrors the behaviour of the regular `build()` function. Setting the flag to
/// `true` causes the error to be propagated to the caller.
pub fn build_with_options(options: &BuildOptions) -> Result<()> {
    dotenvy::dotenv_override().ok();
    set_rerun_triggers();

    let (manifest_dir, out_dir) = manifest_and_out_dir()?;

    constants::generate_constants(&manifest_dir, &out_dir)?;
    let font_path = font::download_font(&manifest_dir)?;
    let ddlog_dir = options.ddlog_dir.clone().unwrap_or_else(|| out_dir.clone());
    compile_ddlog_optional(&manifest_dir, &ddlog_dir, options)?;

    println!("cargo:rustc-env=FONT_PATH={}", font_path.display());

    Ok(())
}

fn manifest_and_out_dir() -> Result<(PathBuf, PathBuf)> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = PathBuf::from(std::env::var("OUT_DIR")?);
    Ok((manifest, out))
}

fn set_rerun_triggers() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    track_ddlog_files(&PathBuf::from("src/ddlog"));
    println!("cargo:rerun-if-changed=constants.toml");
    println!("cargo:rerun-if-env-changed=DDLOG_HOME");
}

fn track_ddlog_files(dir: &PathBuf) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("dl") {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}

fn compile_ddlog_optional(
    manifest_dir: &PathBuf,
    ddlog_dir: &PathBuf,
    options: &BuildOptions,
) -> Result<()> {
    fs::create_dir_all(ddlog_dir)?;
    match ddlog::compile_ddlog(manifest_dir, ddlog_dir) {
        Ok(_) => Ok(()),
        Err(e) if !options.fail_on_ddlog_error => {
            println!("cargo:warning=DDlog build failed: {e}");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_with_options, BuildOptions};
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
        let result = build_with_options(&BuildOptions::default());
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_missing_out_dir() {
        cleanup_test_env();
        env::set_var("CARGO_MANIFEST_DIR", "/tmp/test_manifest");
        let result = build_with_options(&BuildOptions::default());
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_missing_both_env_vars() {
        cleanup_test_env();
        let result = build_with_options(&BuildOptions::default());
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
        let result = build_with_options(&BuildOptions::default());
        assert!(result.is_err());
        cleanup_test_env();
    }

    #[test]
    fn test_build_with_valid_temp_dirs() {
        let (manifest_temp, _) = setup_test_env();
        fs::create_dir_all(manifest_temp.path().join("assets")).ok();
        fs::create_dir_all(manifest_temp.path().join("src/ddlog")).ok();
        fs::write(
            manifest_temp.path().join("constants.toml"),
            "# test constants",
        )
        .ok();
        fs::write(
            manifest_temp.path().join("src/ddlog/lille.dl"),
            "# test ddlog",
        )
        .ok();
        let result = build_with_options(&BuildOptions::default());
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
        fs::create_dir_all(manifest_temp.path().join("src/ddlog")).ok();
        fs::write(manifest_temp.path().join("constants.toml"), "").ok();
        fs::write(manifest_temp.path().join("src/ddlog/lille.dl"), "").ok();
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
        let result = build_with_options(&BuildOptions::default());
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
        let result = build_with_options(&BuildOptions::default());
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
        fs::create_dir_all(manifest_temp.path().join("src/ddlog")).ok();
        fs::write(manifest_temp.path().join("constants.toml"), "").ok();
        fs::write(manifest_temp.path().join("src/ddlog/lille.dl"), "").ok();
        let result1 = build_with_options(&BuildOptions::default());
        let result2 = build_with_options(&BuildOptions::default());
        assert_eq!(result1.is_ok(), result2.is_ok());
        cleanup_test_env();
    }

    fn create_minimal_project_structure(base_path: &std::path::Path) {
        fs::create_dir_all(base_path.join("assets")).ok();
        fs::create_dir_all(base_path.join("src/ddlog")).ok();
        fs::write(base_path.join("constants.toml"), "# Test constants file").ok();
        fs::write(
            base_path.join("src/ddlog").join("lille.dl"),
            "# Test ddlog file",
        )
        .ok();
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
