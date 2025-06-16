use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::NamedTempFile;

pub const DEFAULT_FALLBACK_FONT_PATH: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

fn fallback_font_path() -> PathBuf {
    if let Ok(p) = std::env::var("FALLBACK_FONT_PATH") {
        return PathBuf::from(p);
    }
    #[cfg(target_os = "macos")]
    {
        return PathBuf::from("/System/Library/Fonts/SFNS.ttf");
    }
    #[cfg(target_os = "windows")]
    {
        return PathBuf::from("C:\\Windows\\Fonts\\arial.ttf");
    }
    PathBuf::from(DEFAULT_FALLBACK_FONT_PATH)
}

pub fn download_font(manifest_dir: impl AsRef<Path>) -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = manifest_dir.as_ref();
    let assets_dir = manifest_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;
    let font_path = assets_dir.join("FiraSans-Regular.ttf");

    if font_path.exists() {
        return Ok(font_path);
    }

    match fetch_font_data() {
        Ok(data) => {
            let mut tmp = NamedTempFile::new_in(&assets_dir)?;
            if let Err(e) = tmp.write_all(&data) {
                println!("cargo:warning=Failed to write font: {}", e);
                return Ok(fallback_font_path());
            }
            if let Err(e) = tmp.persist(&font_path) {
                println!("cargo:warning=Failed to rename font file: {}", e);
                return Ok(fallback_font_path());
            }
            Ok(font_path)
        }
        Err(e) => {
            println!("cargo:warning=Font download failed: {}", e);
            Ok(fallback_font_path())
        }
    }
}

fn fetch_font_data() -> Result<Vec<u8>, Box<dyn Error>> {
    const FONT_URL: &str = "https://raw.githubusercontent.com/mozilla/Fira/fd8c8c0a3d353cd99e8ca1662942d165e6961407/ttf/FiraSans-Regular.ttf";
    const FONT_SHA256: &str = "a389cef71891df1232370fcebd7cfde5f74e741967070399adc91fd069b2094b";
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("lille-build/1.0")
        .build()?;
    let resp = client.get(FONT_URL).send()?.error_for_status()?;
    let bytes = resp.bytes()?;
    let digest = Sha256::digest(&bytes);
    let actual = format!("{:x}", digest);
    if actual != FONT_SHA256 {
        return Err("font checksum mismatch".into());
    }
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    #[test]
    fn test_download_font_creates_assets_directory() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let _ = download_font(manifest_path);
        let assets_path = temp_dir.path().join("assets");
        assert!(assets_path.exists(), "Assets directory should be created");
        assert!(assets_path.is_dir(), "Assets path should be a directory");
    }

    #[test]
    fn test_download_font_returns_existing_font_path() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let assets_dir = temp_dir.path().join("assets");
        let font_path = assets_dir.join("FiraSans-Regular.ttf");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(&font_path, b"fake font data").unwrap();
        let result = download_font(manifest_path).unwrap();
        assert_eq!(result, font_path);
        assert!(result.exists());
    }

    #[test]
    fn test_download_font_fallback_on_write_error() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let assets_dir = temp_dir.path().join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        let mut perms = fs::metadata(&assets_dir).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&assets_dir, perms).unwrap();
        let result = download_font(manifest_path).unwrap();
        assert!(result == fallback_font_path() || result.exists());
    }

    #[test]
    fn test_download_font_invalid_manifest_dir() {
        let result = download_font("/non/existent/path");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path == fallback_font_path() || path.ends_with("FiraSans-Regular.ttf"));
    }

    #[test]
    fn test_download_font_empty_manifest_dir() {
        let result = download_font("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_download_font_relative_path() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let result = download_font(manifest_path);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists() || path == fallback_font_path());
    }

    #[test]
    fn test_fallback_font_path_constant() {
        assert_eq!(
            DEFAULT_FALLBACK_FONT_PATH,
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
        );
        assert!(!DEFAULT_FALLBACK_FONT_PATH.is_empty());
    }

    #[test]
    fn test_assets_directory_structure() {
        let temp_dir = setup_test_dir();
        let manifest_path = temp_dir.path().to_str().unwrap();
        let _ = download_font(manifest_path);
        let assets_path = temp_dir.path().join("assets");
        let expected_font_path = assets_path.join("FiraSans-Regular.ttf");
        assert!(assets_path.exists());
        assert_eq!(expected_font_path.parent().unwrap(), assets_path);
    }

    #[test]
    fn test_download_font_creates_nested_directories() {
        let temp_dir = setup_test_dir();
        let nested_manifest = temp_dir
            .path()
            .join("deeply")
            .join("nested")
            .join("manifest");
        fs::create_dir_all(&nested_manifest).unwrap();
        let manifest_path = nested_manifest.to_str().unwrap();
        let result = download_font(manifest_path);
        assert!(result.is_ok());
        let assets_path = nested_manifest.join("assets");
        assert!(assets_path.exists());
    }

    #[test]
    fn test_download_font_concurrent_calls() {
        use std::sync::Arc;
        use std::thread;
        let temp_dir = Arc::new(setup_test_dir());
        let manifest_path = temp_dir.path().to_str().unwrap().to_string();
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let path = manifest_path.clone();
                thread::spawn(move || download_font(&path).is_ok())
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        for result in results {
            assert!(result);
        }
    }
}
