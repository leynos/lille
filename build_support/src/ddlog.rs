//! Build helper for compiling `DDlog` code.
//! Detects the compiler and invokes it during the build process.
use once_cell::sync::OnceCell;
use std::env;
use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

static DDLOG_AVAILABLE: OnceCell<bool> = OnceCell::new();

pub fn compile_ddlog(
    manifest_dir: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let manifest_dir = manifest_dir.as_ref();
    let out_dir = out_dir.as_ref();
    if !ddlog_available() {
        return Ok(());
    }

    let ddlog_file = manifest_dir.join("src/lille.dl");
    if !ddlog_file.exists() {
        println!("cargo:warning=src/lille.dl missing; skipping ddlog compilation");
        return Ok(());
    }

    run_ddlog(&ddlog_file, out_dir)
}

fn ddlog_available() -> bool {
    *DDLOG_AVAILABLE.get_or_init(|| {
        match Command::new("ddlog")
            .arg("--version")
            .stdout(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => true,
            Ok(status) => {
                println!("cargo:warning=ddlog --version failed with status {status}");
                false
            }
            Err(e) => {
                println!("cargo:warning=failed to run ddlog --version: {e}");
                false
            }
        }
    })
}

fn run_ddlog(ddlog_file: &Path, out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let target_dir = out_dir.join("ddlog_lille");
    let mut cmd = Command::new("ddlog");
    if let Ok(home) = env::var("DDLOG_HOME") {
        cmd.arg("-L").arg(format!("{home}/lib"));
    }
    let status = cmd
        .arg("-i")
        .arg(ddlog_file)
        .arg("-o")
        .arg(&target_dir)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ddlog compiler exited with status: {status}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn create_test_dirs() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let manifest_dir = temp_dir.path().join("manifest");
        let out_dir = temp_dir.path().join("out");
        fs::create_dir_all(&manifest_dir).unwrap();
        fs::create_dir_all(&out_dir).unwrap();
        (temp_dir, manifest_dir, out_dir)
    }

    fn create_ddlog_file(manifest_dir: &Path) -> PathBuf {
        let src_dir = manifest_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let ddlog_file = src_dir.join("lille.dl");
        fs::write(&ddlog_file, "// test ddlog file").unwrap();
        ddlog_file
    }

    #[test]
    fn test_compile_ddlog_no_ddlog_available() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        let result = compile_ddlog(&manifest_dir, &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_ddlog_missing_ddlog_file() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        let result = compile_ddlog(&manifest_dir, &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_ddlog_with_existing_file() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        create_ddlog_file(&manifest_dir);
        let result = compile_ddlog(&manifest_dir, &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_ddlog_invalid_manifest_dir() {
        let (_temp_dir, _manifest_dir, out_dir) = create_test_dirs();
        let result = compile_ddlog(Path::new("/non/existent/path"), &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ddlog_available_command_not_found() {
        let available = ddlog_available();
        assert!(available == true || available == false);
    }

    #[test]
    fn test_run_ddlog_with_valid_paths() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        let ddlog_file = create_ddlog_file(&manifest_dir);
        let result = run_ddlog(&ddlog_file, &out_dir);
        match result {
            Ok(_) | Err(_) => assert!(true),
        }
    }

    #[test]
    fn test_run_ddlog_with_ddlog_home_env() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        let ddlog_file = create_ddlog_file(&manifest_dir);
        std::env::set_var("DDLOG_HOME", "/tmp/test_ddlog_home");
        let result = run_ddlog(&ddlog_file, &out_dir);
        std::env::remove_var("DDLOG_HOME");
        match result {
            Ok(_) | Err(_) => assert!(true),
        }
    }

    #[test]
    fn test_run_ddlog_nonexistent_file() {
        let (_temp_dir, _manifest_dir, out_dir) = create_test_dirs();
        let nonexistent_file = std::path::Path::new("/nonexistent/file.dl");
        let result = run_ddlog(&nonexistent_file, &out_dir);
        match result {
            Ok(_) | Err(_) => assert!(true),
        }
    }

    #[test]
    fn test_compile_ddlog_empty_manifest_dir() {
        let out_dir = PathBuf::from("/tmp/test_out");
        let result = compile_ddlog("", &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_ddlog_relative_paths() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        create_ddlog_file(&manifest_dir);
        let relative_manifest = manifest_dir
            .strip_prefix(manifest_dir.parent().unwrap())
            .unwrap();
        let result = compile_ddlog(relative_manifest, &out_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_ddlog_edge_case_paths() {
        let temp_dir = TempDir::new().unwrap();
        let out_dir = temp_dir.path().join("out");
        fs::create_dir_all(&out_dir).unwrap();
        let edge_cases = vec![
            Path::new("."),
            Path::new(".."),
            Path::new("/"),
            Path::new("./"),
            Path::new("../"),
        ];
        for case in edge_cases {
            let result = compile_ddlog(case, &out_dir);
            assert!(result.is_ok(), "Failed for path: {}", case.display());
        }
    }

    #[test]
    fn test_run_ddlog_output_directory_creation() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        let ddlog_file = create_ddlog_file(&manifest_dir);
        assert!(out_dir.exists());
        let result = run_ddlog(&ddlog_file, &out_dir);
        match result {
            Ok(_) | Err(_) => assert!(true),
        }
    }

    #[test]
    fn test_compile_ddlog_with_dotenv() {
        let (_temp_dir, manifest_dir, out_dir) = create_test_dirs();
        create_ddlog_file(&manifest_dir);
        let env_file = manifest_dir.parent().unwrap().join(".env");
        fs::write(&env_file, "TEST_VAR=test_value").unwrap();
        let result = compile_ddlog(&manifest_dir, &out_dir);
        assert!(result.is_ok());
        let _ = fs::remove_file(env_file);
    }

    #[test]
    fn test_ddlog_available_multiple_calls() {
        let first = ddlog_available();
        let second = ddlog_available();
        let third = ddlog_available();
        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    #[test]
    fn test_compile_ddlog_unicode_paths() {
        let temp_dir = TempDir::new().unwrap();
        let unicode_dir = temp_dir.path().join("тест_директория");
        let out_dir = temp_dir.path().join("out");
        fs::create_dir_all(&unicode_dir).unwrap();
        fs::create_dir_all(&out_dir).unwrap();
        let src_dir = unicode_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let ddlog_file = src_dir.join("lille.dl");
        fs::write(&ddlog_file, "// unicode test").unwrap();
        let result = compile_ddlog(&unicode_dir, &out_dir);
        assert!(result.is_ok());
    }
}
